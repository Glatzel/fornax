use std::path::PathBuf;

use fornax::Fornax;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
fn main() -> miette::Result<()> {
    tracing_subscriber::registry()
        .with(clerk::terminal_layer(LevelFilter::DEBUG, true))
        .init();
    let mut manager = Fornax::new(libraw::Libraw::new(), fornax::NullPostProcessor {});
    manager.decode_file(&PathBuf::from(
        "./external/raw-images/images/colorchart-eos-7d.cr2",
    ))?;

    let iparams = manager.decoder.iparams()?;
    println!("{:?}", iparams);
    Ok(())
}
