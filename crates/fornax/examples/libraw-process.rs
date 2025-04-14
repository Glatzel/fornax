use std::path::PathBuf;

use fornax::Fornax;
use libraw::dcraw::DCRawParams;
use miette::IntoDiagnostic;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
fn main() -> miette::Result<()> {
    tracing_subscriber::registry()
        .with(clerk::terminal_layer(LevelFilter::DEBUG, true))
        .init();
    let dcraw_params = DCRawParams {
        user_qual: Some(libraw::dcraw::DCRawUserQual::Linear),
        ..Default::default()
    };
    let mut manager = Fornax::new(
        libraw::Libraw::new(),
        libraw::dcraw::DCRaw::new(dcraw_params),
    );
    let img = manager
        .decode_file(&PathBuf::from(
            "./external/raw-images/images/colorchart-eos-7d.cr2",
        ))?
        .post_process()?
        .to_dynamic_image();
    img.save("temp/example-process.tiff").into_diagnostic()?;
    clerk::info!("save img to: temp/example-process.tiff");
    Ok(())
}
