use std::{path::PathBuf, str::Utf8Error};

use fornax::Fornax;
use miette::IntoDiagnostic;
mod utils;
fn main() -> miette::Result<()> {
    utils::example_setup();

    default_path()?;
    custom_path()?;
    Ok(())
}

fn default_path() -> miette::Result<()> {
    let dnc = dnc::Dnc::new(dnc::DncParams {
        overwrite: true,
        ..Default::default()
    });

    let dng_file = dnc.convert(&utils::raw_file())?;
    let libraw = libraw::Libraw::default();
    let mut manager = Fornax::new(&libraw, &libraw);
    let img = manager
        .decode_file(&dng_file)?
        .post_process()?
        .to_dynamic_image();
    img.save("temp/dng-converter.tiff").into_diagnostic()?;
    clerk::info!("save img to: temp/dng-converter.tiff");
    Ok(())
}

fn custom_path() -> miette::Result<()> {
    let dnc = dnc::Dnc::new(dnc::DncParams {
        directory: Some(PathBuf::from("./temp")),
        filename: Some("dng-converter.dng".to_string()),
        overwrite: true,
        ..Default::default()
    });
    let dng_file = dnc.convert(&utils::raw_file())?;
    let libraw = libraw::Libraw::default();
    let mut manager = Fornax::new(&libraw, &libraw);
    let img = manager
        .decode_file(&dng_file)?
        .post_process()?
        .to_dynamic_image();

    img.save(utils::output_dir().join("dnc.tiff"))
        .into_diagnostic()?;
    clerk::info!("save img to: dnc.tiff");
    Ok(())
}
