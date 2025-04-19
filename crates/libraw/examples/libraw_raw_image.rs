use miette::IntoDiagnostic;
mod utils;
fn main() -> miette::Result<()> {
    utils::example_setup();
    let libraw = libraw::Libraw::new(None);
    let img = libraw
        .open_file(&utils::raw_file())?
        .unpack()?
        .get_raw_image(true)?;

    clerk::info!("Done building raw image.");

    img.save(utils::output_dir().join("raw_image.tiff"))
        .into_diagnostic()?;
    clerk::info!("Done saving raw image.");
    Ok(())
}
