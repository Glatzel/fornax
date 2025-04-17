use fornax::Fornax;
use miette::IntoDiagnostic;
mod utils;
fn main() -> miette::Result<()> {
    utils::example_setup();

    let libraw = libraw::Libraw::new(None);
    let mut manager = Fornax::new(&libraw, &libraw);
    manager.decode_file(&utils::raw_file())?;

    let rawdatas = manager.decoder.rawdata()?;
    clerk::info!("Done building raw image.");
    for img in rawdatas {
        match img {
            libraw::libraw::LibrawRawdata::Mono16(image_buffer) => {
                image_buffer
                    .save(utils::output_dir().join("rawdata_mono16.tiff"))
                    .into_diagnostic()?;
                clerk::info!("Found mono16 rawdata.");
                clerk::info!("Done saving raw image.");
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
