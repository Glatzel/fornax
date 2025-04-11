use std::path::PathBuf;

use fornax::Fornax;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;
fn main() -> miette::Result<()> {
    tracing_subscriber::registry()
        .with(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::DEBUG.into())
                .from_env_lossy(),
        )
        .with(clerk::terminal_layer())
        .init();
    let mut manager = Fornax::new(libraw::Libraw::new(), fornax::NullPostProcessor {});
    manager.decode_file(&PathBuf::from(
        "./external/raw-images/images/colorchart-eos-7d.cr2",
    ))?;

    let iparams = manager.decoder.iparams()?;
    println!("{:?}", iparams);
    Ok(())
}
