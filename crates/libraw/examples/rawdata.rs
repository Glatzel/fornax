use miette::IntoDiagnostic;

fn main() -> miette::Result<()> {
    fornax_devtool::example_setup();
    let libraw = libraw::Libraw::new(None);
    let img = libraw
        .open_file(&fornax_devtool::raw_file())?
        .unpack()?
        .get_rawdata()?;

    match img {
        libraw::Rawdata::Mono16(image_buffer) => {
            image_buffer
                .save(fornax_devtool::output_dir().join("rawdata_mono16.tiff"))
                .into_diagnostic()?;
            clerk::info!("Found mono16 rawdata.");
            clerk::info!("Done saving raw image.");
        }
        libraw::Rawdata::Rgb16(_) => {
            clerk::info!("Found rgb16 rawdata.")
        }
        libraw::Rawdata::Rgba16(_) => {
            clerk::info!("Found rgba16 rawdata.")
        }
        libraw::Rawdata::MonoF32(_) => {
            clerk::info!("Found mono32 rawdata.")
        }
        libraw::Rawdata::RgbF32(_) => {
            clerk::info!("Found rgb32 rawdata.")
        }
        libraw::Rawdata::RgbaF32(_) => {
            clerk::info!("Found rgba32 rawdata.")
        }
    }

    Ok(())
}
