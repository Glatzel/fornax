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
    let mut processor = Fornax::new();
    processor
        .open_file(PathBuf::from(
            "./external/raw-images/images/colorchart-5D2-6000K.dng",
        ))
        .unwrap();
    processor.unpack().unwrap();

    let sizes = processor.image_sizes()?;
    println!("{:?}", sizes);
    Ok(())
}
