use std::path::PathBuf;

use fornax::Fornax;
use miette::IntoDiagnostic;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
fn main() -> miette::Result<()> {
    tracing_subscriber::registry()
        .with(clerk::terminal_layer(LevelFilter::DEBUG, true))
        .init();
    default_path()?;
    custom_path()?;
    Ok(())
}
fn default_path() -> miette::Result<()> {
    let mut manager = Fornax::new(
        dnc::Dnc::new(dnc::DncParams {
            overwrite: true,
            ..Default::default()
        }),
        libraw::dcraw::DCRaw::default(),
    );
    let img = manager
        .decode_file(&PathBuf::from(
            "./external/raw-images/images/colorchart-eos-7d.cr2",
        ))?
        .post_process()?
        .to_dynamic();

    img.save("temp/dng-converter.tiff").into_diagnostic()?;
    clerk::info!("save img to: temp/dng-converter.tiff");
    Ok(())
}
fn custom_path() -> miette::Result<()> {
    let mut manager = Fornax::new(
        dnc::Dnc::new(dnc::DncParams {
            directory: Some(PathBuf::from("./temp")),
            filename: Some("dng-converter.dng".to_string()),
            overwrite: true,
            ..Default::default()
        }),
        libraw::dcraw::DCRaw::default(),
    );
    let img = manager
        .decode_file(&PathBuf::from(
            "./external/raw-images/images/colorchart-eos-7d.cr2",
        ))?
        .post_process()?
        .to_dynamic();

    img.save("temp/dng-converter.tiff").into_diagnostic()?;
    clerk::info!("save img to: temp/dng-converter.tiff");
    Ok(())
}
