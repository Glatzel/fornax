use std::path::PathBuf;

use fornax::Fornax;
use libraw::libraw::LibrawRawdata;
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
        "./external/raw-images/images/colorchart-eos-7d.cr2",
    ))?;

    manager
        .decoder
        .rawdata(&LibrawRawdata::RawImage)?
        .to_dynamic_image()
        .save("temp/raw_mono16.tiff")
        .into_diagnostic()?;
    manager
        .decoder
        .rawdata(&LibrawRawdata::Color3Image)?
        .to_dynamic_image()
        .save("temp/raw_rgb16.tiff")
        .into_diagnostic()?;
    manager
        .decoder
        .rawdata(&LibrawRawdata::Color4Image)?
        .to_dynamic_image()
        .save("temp/raw_rgba.tiff")
        .into_diagnostic()?;
    manager
        .decoder
        .rawdata(&LibrawRawdata::FloatImage)?
        .to_dynamic_image()
        .save("temp/raw_mono16.tiff")
        .into_diagnostic()?;
    manager
        .decoder
        .rawdata(&LibrawRawdata::Float3Image)?
        .to_dynamic_image()
        .save("temp/raw_mono16.tiff")
        .into_diagnostic()?;
    manager
        .decoder
        .rawdata(&LibrawRawdata::Float4Image)?
        .to_dynamic_image()
        .save("temp/raw_mono16.tiff")
        .into_diagnostic()?;

    Ok(())
}
