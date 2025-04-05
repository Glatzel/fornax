use std::path::PathBuf;

use fornax::Fornax;
use miette::IntoDiagnostic;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
fn main() -> miette::Result<()> {
    tracing_subscriber::registry()
        .with(clerk::terminal_layer(LevelFilter::DEBUG))
        .init();
    let mut manager = Fornax::new(libraw::Libraw::new(), dcraw::DCRaw::default());
    let img = manager
        .decode_file(PathBuf::from(
            "./external/raw-images/images/colorchart-5D2-6000K.dng",
        ))?
        .post_process()?
        .to_dynamic();
    img.save("temp/example-process.tiff").into_diagnostic()?;
    clerk::info!("save img to: temp/example-process.tiff");
    Ok(())
}
