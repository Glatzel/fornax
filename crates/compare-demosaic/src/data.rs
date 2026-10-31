use std::path::Path;

use fornax_core::BayerPattern;
use fornax_core::protocol::{IDecoder, IPreprocessor};
use fornax_decoder_rawler::RawlerDecoder;
use fornax_preprocessor::Preprocessor;

#[derive(Clone, Debug)]
pub struct ActiveArea {
    pub source: String,
    pub width: u32,
    pub height: u32,
    pub codes: Vec<u16>,
    pub native_pattern: BayerPattern,
}

#[cfg_attr(
    not(test),
    allow(dead_code, reason = "used by comparison crate integration tests")
)]
pub fn active_area() -> mischief::Result<ActiveArea> {
    active_area_from(&fornax_devtool::raw_file())
}

pub fn active_area_from(path: &Path) -> mischief::Result<ActiveArea> {
    let (decoded, metadata) =
        <RawlerDecoder as IDecoder<Path, f32>>::decode_active(&RawlerDecoder, path)
            .map_err(|error| mischief::mischief!("cannot decode {}: {error}", path.display()))?;
    let image = Preprocessor
        .process(decoded, &metadata.levels)
        .map_err(|error| mischief::mischief!("cannot preprocess {}: {error}", path.display()))?
        .into_image();
    let (width, height) = image.dimensions();
    Ok(ActiveArea {
        source: path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        width,
        height,
        codes: image
            .as_raw()
            .iter()
            .map(|sample| (sample.clamp(0.0, 1.0) * f32::from(u16::MAX)).round() as u16)
            .collect(),
        native_pattern: metadata.bayer_pattern,
    })
}
