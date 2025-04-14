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
    let params = libraw::dcraw::DCRawParams::preset_cg();
    let libraw = libraw::Libraw::new(Some(params));
    let mut manager = Fornax::new(&libraw, &libraw);
    let img = manager
        .decode_file(&PathBuf::from(
            "./external/raw-images/images/colorchart-eos-7d.cr2",
        ))?
        .post_process()?
        .to_dynamic_image();
    img.save("temp/example-process-cg.tiff").into_diagnostic()?;
    clerk::info!("save img to: temp/example-process.tiff");
    Ok(())
}
