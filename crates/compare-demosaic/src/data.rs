use std::path::{Path, PathBuf};

use fornax_core::BayerPattern;
use fornax_core::protocol::{IDecoder, IPreprocessor};
use fornax_decoder_rawler::RawlerDecoder;
use fornax_preprocessor::Preprocessor;

pub const CANONICAL_SOURCE: &str = "external/raw-images/images/colorchart-eos-7d.cr2";
pub const CANONICAL_WIDTH: u32 = 5_202;
pub const CANONICAL_HEIGHT: u32 = 3_464;
pub const CANONICAL_PATTERN: BayerPattern = BayerPattern::GBRG;

#[derive(Clone, Debug)]
pub struct Input {
    pub source: String,
    pub pattern: BayerPattern,
    pub width: u32,
    pub height: u32,
    pub codes: Vec<u16>,
}

pub fn canonical() -> mischief::Result<Input> {
    let path = workspace_root().join(CANONICAL_SOURCE);
    decode_input(&path)
        .map_err(|error| mischief::mischief!("cannot decode {}: {error}", path.display()))
}

#[cfg(test)]
pub fn canonical_crop() -> mischief::Result<Input> {
    let input = canonical()?;
    let width = 512;
    let height = 512;
    // Even origin offsets preserve the original 2x2 CFA phase.
    let x = (input.width - width) / 4 * 2;
    let y = (input.height - height) / 2;
    let codes = (y..y + height)
        .flat_map(|row| {
            let start = row as usize * input.width as usize + x as usize;
            input.codes[start..start + width as usize].iter().copied()
        })
        .collect();
    Ok(Input {
        source: input.source,
        pattern: input.pattern,
        width,
        height,
        codes,
    })
}

fn workspace_root() -> PathBuf { PathBuf::from(env!("CARGO_WORKSPACE_DIR")) }

fn decode_input(path: &Path) -> Result<Input, String> {
    let (decoded, metadata) =
        <RawlerDecoder as IDecoder<Path, f32>>::decode_active(&RawlerDecoder, path)
            .map_err(|error| error.to_string())?;
    let normalized = Preprocessor
        .process(decoded, &metadata.levels)
        .map_err(|error| error.to_string())?
        .into_image();
    if metadata.bayer_pattern != CANONICAL_PATTERN {
        return Err(format!(
            "expected {} CFA metadata, got {}",
            CANONICAL_PATTERN, metadata.bayer_pattern
        ));
    }
    let (width, height) = normalized.dimensions();
    if (width, height) != (CANONICAL_WIDTH, CANONICAL_HEIGHT) {
        return Err(format!(
            "expected full active area {CANONICAL_WIDTH}x{CANONICAL_HEIGHT}, got {width}x{height}"
        ));
    }
    Ok(Input {
        source: CANONICAL_SOURCE.to_owned(),
        pattern: metadata.bayer_pattern,
        width,
        height,
        codes: normalized.as_raw().iter().copied().map(quantize).collect(),
    })
}

fn quantize(sample: f32) -> u16 { (sample.clamp(0.0, 1.0) * f32::from(u16::MAX)).round() as u16 }

#[cfg(test)]
pub fn test_inputs() -> Vec<Input> {
    vec![
        seeded("canonical", 512, 512),
        seeded("odd", 513, 511),
        seeded("partial-row-chunk", 34, 513),
        seeded("tiny", 3, 3),
        saturated("saturated", 512, 512),
        saturated("saturated-tiny", 3, 3),
    ]
}

#[cfg(test)]
pub(crate) fn seeded(label: &str, width: u32, height: u32) -> Input {
    let mut state = 0x1234_5678_u64;
    let codes = (0..width as usize * height as usize)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state >> 48) as u16
        })
        .collect();
    Input {
        source: label.to_owned(),
        pattern: BayerPattern::GBRG,
        width,
        height,
        codes,
    }
}

#[cfg(test)]
pub(crate) fn saturated(label: &str, width: u32, height: u32) -> Input {
    Input {
        source: label.to_owned(),
        pattern: BayerPattern::GBRG,
        width,
        height,
        codes: vec![u16::MAX; width as usize * height as usize],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_cr2_uses_the_full_active_area_and_raw_cfa() {
        let input = canonical().expect("the pinned CR2 fixture must decode");
        assert_eq!(input.source, CANONICAL_SOURCE);
        assert_eq!(input.pattern, BayerPattern::GBRG);
        assert_eq!((input.width, input.height), (5_202, 3_464));
        assert_eq!(
            input.codes.len(),
            CANONICAL_WIDTH as usize * CANONICAL_HEIGHT as usize
        );
    }

    #[test]
    fn canonical_test_crop_is_512_by_512_and_preserves_the_cfa() {
        let input = canonical_crop().expect("the pinned CR2 fixture must decode");
        assert_eq!(input.pattern, BayerPattern::GBRG);
        assert_eq!((input.width, input.height), (512, 512));
        assert_eq!(input.codes.len(), 512 * 512);
    }

    #[test]
    fn normalized_samples_are_quantized_and_clamped() {
        assert_eq!(quantize(-1.0), 0);
        assert_eq!(quantize(0.0), 0);
        assert_eq!(quantize(1.0), u16::MAX);
        assert_eq!(quantize(2.0), u16::MAX);
        assert_eq!(quantize(0.5), 32_768);
    }
}
