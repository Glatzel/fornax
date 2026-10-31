use std::path::{Path, PathBuf};

use fornax_core::BayerPattern;
use fornax_core::protocol::{IDecoder, IPreprocessor};
use fornax_decoder_rawler::RawlerDecoder;
#[cfg(test)]
use fornax_devtool::synthetic_mono_image;
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
    let path = PathBuf::from(env!("CARGO_WORKSPACE_DIR")).join(CANONICAL_SOURCE);
    decode_input(&path)
        .map_err(|error| mischief::mischief!("cannot decode {}: {error}", path.display()))
}

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
pub fn synthetic_input() -> Input {
    let image = synthetic_mono_image::<f32>((512, 512), 65_535.0);
    Input {
        source: "fornax-devtool::synthetic_mono_image".to_owned(),
        pattern: BayerPattern::GBRG,
        width: 512,
        height: 512,
        codes: image
            .as_raw()
            .iter()
            .map(|sample| sample.round().clamp(0.0, 65_535.0) as u16)
            .collect(),
    }
}
