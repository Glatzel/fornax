use std::path::PathBuf;

use fornax::Fornax;
use fornax_core::BayerPattern;
use miette::IntoDiagnostic;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
fn main() -> miette::Result<()> {
    tracing_subscriber::registry()
        .with(clerk::terminal_layer(LevelFilter::DEBUG, true))
        .init();
    let libraw = libraw::Libraw::new(None);
    let mut manager = Fornax::new(&libraw, &libraw);
    manager.decode_file(&PathBuf::from(
        "./external/raw-images/images/colorchart-eos-7d.cr2",
    ))?;
    let bayer_pattern = manager.decoder.bayer_pattern()?;
    clerk::info!("Bayer pattern: {}", bayer_pattern);
    assert_eq!(bayer_pattern, BayerPattern::GBRG);
    let bayer_image = manager.decoder.get_bayer_image()?;
    assert_eq!(bayer_image.bayer_image().width(), 5202);
    assert_eq!(bayer_image.bayer_image().height(), 3464);
    bayer_image
        .bayer_image()
        .save("temp/bayerimga.tiff")
        .into_diagnostic()?;
    clerk::info!("save img to: temp/bayerimga.tiff");
    Ok(())
}
