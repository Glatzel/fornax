use std::path::Path;

use fornax::decoder::{IRawDecoder, LibRawDecoder};
use fornax::{DecodedRaw, DemosaicAlgorithm, StoredPlane};
use image::{ImageBuffer, Luma};

fn main() -> mischief::Result<()> {
    let output_dir = fornax_devtool::temp_dir();
    std::fs::create_dir_all(&output_dir)?;

    let decoder = LibRawDecoder::new();
    let raw_path = fornax_devtool::raw_file();
    let decoded = decoder.decode(raw_path.as_path())?;
    let dimensions = decoded.plane().dimensions()?;
    assert_eq!(
        (dimensions.width(), dimensions.height()),
        (5_360, 3_516),
        "decoded output retains the stored raster"
    );
    let decoded_path = output_dir.join("fornax-libraw-decoded.tiff");
    save_decoded(&decoded, &decoded_path)?;

    let preprocessed = decoded.preprocess()?;
    assert_eq!(
        preprocessed.plane().dimensions(),
        (5_202, 3_464),
        "preprocessed output is cropped to ActiveArea"
    );
    let preprocessed_path = output_dir.join("fornax-libraw-preprocess.tiff");
    save_f32_gray(preprocessed.plane(), &preprocessed_path)?;

    let (demosaiced, _) = preprocessed.demosaic(DemosaicAlgorithm::Linear)?;
    assert_eq!(
        demosaiced.dimensions(),
        (5_202, 3_464),
        "demosaiced output matches the active mosaic"
    );
    let demosaiced_path = output_dir.join("fornax-libraw-demosaic_linear.tiff");
    save_f32_rgb(&demosaiced, &demosaiced_path)?;

    println!("Decoded output: {}", decoded_path.display());
    println!("Preprocessed output: {}", preprocessed_path.display());
    println!("Demosaiced output: {}", demosaiced_path.display());
    Ok(())
}

fn save_decoded(decoded: &DecodedRaw, path: &Path) -> mischief::Result<()> {
    match decoded.plane() {
        StoredPlane::U8(plane) => plane.save(path)?,
        StoredPlane::U16(plane) => plane.save(path)?,
        StoredPlane::U32(plane) => {
            let (width, height) = plane.dimensions();
            ImageBuffer::<Luma<f32>, Vec<f32>>::from_raw(
                width,
                height,
                plane.as_raw().iter().map(|sample| *sample as f32).collect(),
            )
            .unwrap()
            .save(path)?;
        }
        StoredPlane::F32(plane) => plane.save(path)?,
    }
    Ok(())
}

fn save_f32_gray(plane: &fornax::F32Plane, path: &Path) -> mischief::Result<()> {
    let (width, height) = plane.dimensions();
    ImageBuffer::<Luma<u16>, Vec<u16>>::from_raw(
        width,
        height,
        plane
            .as_raw()
            .iter()
            .map(|sample| (sample.clamp(0.0, 1.0) * f32::from(u16::MAX)).round() as u16)
            .collect(),
    )
    .ok_or_else(|| std::io::Error::other("f32 plane dimensions are inconsistent"))?
    .save(path)?;
    Ok(())
}

fn save_f32_rgb(plane: &fornax::RgbImageF32, path: &Path) -> mischief::Result<()> {
    ImageBuffer::<image::Rgb<u16>, Vec<u16>>::from_raw(
        plane.width(),
        plane.height(),
        plane
            .as_raw()
            .iter()
            .map(|sample| (sample.clamp(0.0, 1.0) * f32::from(u16::MAX)).round() as u16)
            .collect(),
    )
    .ok_or_else(|| std::io::Error::other("RGB dimensions are inconsistent"))?
    .save(path)?;
    Ok(())
}

#[test]
fn libraw_pipeline_example_writes_each_stage() -> mischief::Result<()> {
    main()?;
    for filename in [
        "fornax-libraw-decoded.tiff",
        "fornax-libraw-preprocess.tiff",
        "fornax-libraw-demosaic_linear.tiff",
    ] {
        assert!(fornax_devtool::temp_dir().join(filename).is_file());
    }
    Ok(())
}
