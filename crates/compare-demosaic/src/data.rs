use fornax_core::BayerPattern;
use fornax_core::protocol::{IDecoder, IPreprocessor};
use fornax_decoder_rawler::RawlerDecoder;
#[cfg(test)]
use fornax_devtool::synthetic_mono_image;
use fornax_preprocessor::Preprocessor;

pub const CANONICAL_WIDTH: u32 = 512;
pub const CANONICAL_HEIGHT: u32 = 512;
const REAL_FIXTURE: &str = "external/raw-images/images/colorchart-eos-7d.cr2";

#[derive(Clone, Debug)]
pub struct Input {
    #[allow(dead_code, reason = "used in comparison test diagnostics")]
    pub label: String,
    pub source: String,
    pub kind: String,
    pub pattern: Option<BayerPattern>,
    pub width: u32,
    pub height: u32,
    pub codes: Vec<u16>,
}

pub fn canonical() -> Input {
    let path = fornax_devtool::root_dir().join(REAL_FIXTURE);
    assert!(
        path.is_file(),
        "real comparison fixture is missing: {}; run the platform setup script",
        path.display()
    );
    let (decoded, metadata) = <RawlerDecoder as IDecoder<std::path::Path, f32>>::decode_active(
        &RawlerDecoder,
        path.as_path(),
    )
    .expect("decode real comparison fixture");
    let normalized = Preprocessor
        .process(decoded, &metadata.levels)
        .expect("normalize real comparison fixture")
        .into_image();
    let (width, height) = normalized.dimensions();
    assert!(width >= CANONICAL_WIDTH && height >= CANONICAL_HEIGHT);
    let x = ((width - CANONICAL_WIDTH) / 2) & !1;
    let y = ((height - CANONICAL_HEIGHT) / 2) & !1;
    let row_stride = width as usize;
    let mut crop = Vec::with_capacity((CANONICAL_WIDTH * CANONICAL_HEIGHT) as usize);
    for row in y..y + CANONICAL_HEIGHT {
        let start = row as usize * row_stride + x as usize;
        crop.extend_from_slice(&normalized.as_raw()[start..start + CANONICAL_WIDTH as usize]);
    }
    Input {
        label: "real-colorchart-center".to_owned(),
        source: REAL_FIXTURE.to_owned(),
        kind: "real_raw".to_owned(),
        pattern: Some(metadata.bayer_pattern),
        width: CANONICAL_WIDTH,
        height: CANONICAL_HEIGHT,
        codes: crop
            .iter()
            .map(|sample| (sample.clamp(0.0, 1.0) * f32::from(u16::MAX)).round() as u16)
            .collect(),
    }
}

#[cfg(test)]
pub fn test_inputs() -> Vec<Input> {
    vec![
        seeded("canonical", CANONICAL_WIDTH, CANONICAL_HEIGHT),
        seeded("odd", 513, 511),
        seeded("partial-row-chunk", 34, 513),
        seeded("tiny", 3, 3),
        saturated("saturated", CANONICAL_WIDTH, CANONICAL_HEIGHT),
        saturated("saturated-tiny", 3, 3),
    ]
}

#[cfg(test)]
fn seeded(label: &str, width: u32, height: u32) -> Input {
    let image = synthetic_mono_image::<f32>((width, height), 65_535.0);
    Input {
        label: label.to_owned(),
        source: "fornax-devtool::synthetic_mono_image".to_owned(),
        kind: "synthetic_stress".to_owned(),
        pattern: None,
        width,
        height,
        codes: image
            .as_raw()
            .iter()
            .map(|sample| sample.round().clamp(0.0, 65_535.0) as u16)
            .collect(),
    }
}

#[cfg(test)]
fn saturated(label: &str, width: u32, height: u32) -> Input {
    Input {
        label: label.to_owned(),
        source: "constant".to_owned(),
        kind: "synthetic_stress".to_owned(),
        pattern: None,
        width,
        height,
        codes: vec![u16::MAX; width as usize * height as usize],
    }
}
