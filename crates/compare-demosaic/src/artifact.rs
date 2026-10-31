use std::path::PathBuf;

use fornax_core::BayerPattern;
use image::{GrayImage, RgbImage};

use crate::case::Case;

pub fn write_failure(
    case: &Case,
    pattern: BayerPattern,
    reference: &[u16],
    actual: &[u16],
    width: u32,
    height: u32,
) -> mischief::Result<()> {
    if std::env::var_os("FORNAX_COMPARE_ARTIFACTS").is_none() {
        return Ok(());
    }
    let directory = PathBuf::from(env!("CARGO_WORKSPACE_DIR"))
        .join("target/compare-demosaic")
        .join(case.name)
        .join(pattern.to_string());
    std::fs::create_dir_all(&directory)?;

    let rgb = |codes: &[u16]| {
        RgbImage::from_raw(
            width,
            height,
            codes
                .iter()
                .map(|code| (code / 257) as u8)
                .collect::<Vec<_>>(),
        )
        .expect("comparison artifact dimensions")
    };
    rgb(reference).save(directory.join("reference.png"))?;
    rgb(actual).save(directory.join("actual.png"))?;
    GrayImage::from_raw(
        width,
        height,
        reference
            .iter()
            .zip(actual)
            .map(|(reference, actual)| {
                u8::try_from(reference.abs_diff(*actual).saturating_mul(16).min(255)).unwrap()
            })
            .collect(),
    )
    .expect("comparison diff dimensions")
    .save(directory.join("diff.png"))?;
    Ok(())
}
