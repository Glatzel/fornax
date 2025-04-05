use fornax::Fornax;
use miette::IntoDiagnostic;

use tracing::level_filters::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
fn main() -> miette::Result<()> {
    tracing_subscriber::registry()
        .with(clerk::terminal_layer(LevelFilter::DEBUG))
        .init();
    let mut manager = Fornax::new(
        dng_converter::DngConverter::default(),
        libraw::dcraw::DCRaw::default(),
    );
    let img = manager
        .decode_file(dunce::canonicalize("./external/a7r5.ARW").unwrap())?
        .post_process()?
        .to_dynamic();
    img.save("temp/dng-converter.tiff").into_diagnostic()?;
    clerk::info!("save img to: temp/dng-converter.tiff");
    Ok(())
}
