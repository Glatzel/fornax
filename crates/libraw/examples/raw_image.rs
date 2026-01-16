fn main() -> mischief::Result<()> {
    fornax_devtool::example_setup();
    let libraw = libraw::Libraw::default();
    let img = libraw
        .open_file(&fornax_devtool::raw_file())?
        .unpack()?
        .get_raw_image(true)?;

    clerk::info!("Done building raw image.");

    img.save(fornax_devtool::output_dir().join("raw_image.tiff"))?;
    clerk::info!("Done saving raw image.");
    Ok(())
}
