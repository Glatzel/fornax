use std::path::PathBuf;

use fornax::Fornax;
use miette::IntoDiagnostic;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
fn main() -> miette::Result<()> {
    tracing_subscriber::registry()
        .with(clerk::terminal_layer(LevelFilter::DEBUG, true))
        .init();
    let mut manager = Fornax::new(libraw::Libraw::new(), fornax::NullPostProcessor {});
    manager.decode_file(&PathBuf::from(
        "./external/raw-images/images/colorchart-iphone7plus-cloudy.dng",
    ))?;

    manager
        .decoder
        .rawimage()?
        .save("temp/raw_image.tiff")
        .into_diagnostic()?;
    Ok(())
}
