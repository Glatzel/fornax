use fornax_core::BayerPattern;
use miette::IntoDiagnostic;

fn main() -> miette::Result<()> {
    fornax_devtool::example_setup();
    let libraw = libraw::Libraw::new(None);
    let img: fornax_core::BayerImage<u16> = libraw
        .open_file(&fornax_devtool::raw_file())?
        .unpack()?
        .get_bayer_image()?;
    let bayer_pattern = img.pattern();
    clerk::info!("Bayer pattern: {}", bayer_pattern);
    assert_eq!(bayer_pattern, &BayerPattern::GBRG);
    let mosaic = img.mosaic();
    assert_eq!(mosaic.width(), 5202);
    assert_eq!(mosaic.height(), 3464);

    mosaic
        .save(fornax_devtool::output_dir().join("bayerimga.tiff"))
        .into_diagnostic()?;
    clerk::info!("Done saving raw image.");
    Ok(())
}
