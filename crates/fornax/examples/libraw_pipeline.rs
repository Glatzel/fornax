use std::path::Path;

use fornax::decoder::{IDecoder, LibRawDecoder};
use fornax::{DecodedRaw, LumaImage, RgbImage};
use image::{ImageBuffer, Luma, Rgb};

fn main() -> mischief::Result<()> {
    let output_dir = fornax_devtool::prepare_output_dir("libraw-pipeline")?;
    let raw_path = fornax_devtool::raw_file();
    let decoded: DecodedRaw<f32> = LibRawDecoder::new().decode(raw_path.as_path())?;

    assert_eq!(decoded.image().dimensions(), (5_360, 3_516));
    assert_eq!(decoded.metadata().preprocess.active_area.width, 5_202);
    assert_eq!(decoded.metadata().preprocess.active_area.height, 3_464);
    let decoded_path = output_dir.join("fornax-libraw-decoded.tiff");
    save_f32_gray(decoded.image(), &decoded_path)?;

    let (processed, metadata) = decoded.preprocess();
    assert_eq!(processed.image().dimensions(), (5_360, 3_516));
    let preprocessed_path = output_dir.join("fornax-libraw-preprocess.tiff");
    save_f32_gray(processed.image(), &preprocessed_path)?;

    let mosaic = processed.cropped(metadata.demosaic.active_area)?;
    let demosaiced = demosaic(mosaic, metadata.demosaic.bayer_pattern);
    assert_eq!(demosaiced.dimensions(), (5_202, 3_464));
    let demosaiced_path = output_dir.join("fornax-libraw-demosaic_linear.tiff");
    save_f32_rgb(&demosaiced, &demosaiced_path)?;

    println!("Decoded output: {}", decoded_path.display());
    println!("Preprocessed output: {}", preprocessed_path.display());
    println!("Demosaiced output: {}", demosaiced_path.display());
    println!(
        "{} {}",
        metadata.decoder.vendor.make.unwrap_or_default(),
        metadata.decoder.vendor.model.unwrap_or_default()
    );
    Ok(())
}

fn save_f32_gray(plane: &LumaImage, path: &Path) -> mischief::Result<()> {
    let (width, height) = plane.dimensions();
    ImageBuffer::<Luma<u16>, Vec<u16>>::from_raw(
        width,
        height,
        plane
            .as_raw()
            .iter()
            .map(|sample| (sample * f32::from(u16::MAX)).round() as u16)
            .collect(),
    )
    .ok_or_else(|| std::io::Error::other("f32 plane dimensions are inconsistent"))?
    .save(path)?;
    Ok(())
}

fn save_f32_rgb(plane: &RgbImage, path: &Path) -> mischief::Result<()> {
    ImageBuffer::<Rgb<u16>, Vec<u16>>::from_raw(
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
fn libraw_pipeline_example_writes_grouped_outputs() -> mischief::Result<()> {
    main()?;
    for filename in [
        "fornax-libraw-decoded.tiff",
        "fornax-libraw-preprocess.tiff",
        "fornax-libraw-demosaic_linear.tiff",
    ] {
        assert!(
            fornax_devtool::output_dir_for("libraw-pipeline", "fornax")
                .join(filename)
                .is_file()
        );
    }
    Ok(())
}
