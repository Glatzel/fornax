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
    let libraw = libraw::Libraw::new(None);
    let mut manager = Fornax::new(&libraw, &libraw);
    manager.decode_file(&PathBuf::from(
        "./external/raw-images/images/colorchart-eos-7d.cr2",
    ))?;

    let rawdatas = manager.decoder.rawdata()?;
    clerk::info!("Done building raw image.");
    for img in rawdatas {
        match img {
            libraw::libraw::LibrawRawdata::Mono16(image_buffer) => {
                image_buffer
                    .save("temp/rawdata_mono16.tiff")
                    .into_diagnostic()?;
                clerk::info!("Found mono16 rawdata.");
            }
            libraw::libraw::LibrawRawdata::Rgb16(_) => {
                clerk::info!("Found rgb16 rawdata.")
            }
            libraw::libraw::LibrawRawdata::Rgba16(_) => {
                clerk::info!("Found rgba16 rawdata.")
            }
            libraw::libraw::LibrawRawdata::MonoF32(_) => {
                clerk::info!("Found mono32 rawdata.")
            }
            libraw::libraw::LibrawRawdata::RgbF32(_) => {
                clerk::info!("Found rgb32 rawdata.")
            }
            libraw::libraw::LibrawRawdata::RgbaF32(_) => {
                clerk::info!("Found rgba32 rawdata.")
            }
        }
    }

    Ok(())
}
