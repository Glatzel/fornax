use std::path::PathBuf;

use fornax::Fornax;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
fn main() -> miette::Result<()> {
    tracing_subscriber::registry()
        .with(clerk::terminal_layer(LevelFilter::DEBUG))
        .init();
    let mut manager = Fornax::new(libraw::Libraw::new(), fornax::NullPostProcessor {});
    manager.decode(PathBuf::from(
        "./external/raw-images/images/colorchart-5D2-6000K.dng",
    ))?;

    let sizes = manager.decoder.image_sizes()?;
    println!("{:?}", sizes);
    Ok(())
}
