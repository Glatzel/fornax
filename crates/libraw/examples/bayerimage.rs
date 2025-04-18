use fornax_core::BayerPattern;
use miette::IntoDiagnostic;
mod utils;
fn main() -> miette::Result<()> {
    utils::example_setup();

    let libraw = libraw::Libraw::new(None);
    let manager: Fornax<&libraw::Libraw, u16, &libraw::Libraw, u16> = Fornax::new(&libraw, &libraw);
    manager.decode_file(&utils::raw_file())?;
    let bayer_pattern = manager.decoder.bayer_pattern()?;
    clerk::info!("Bayer pattern: {}", bayer_pattern);
    assert_eq!(bayer_pattern, BayerPattern::GBRG);
    let bayer_image: fornax_core::BayerImage<u16> = manager.decoder.get_bayer_image()?;
    assert_eq!(bayer_image.mosaic().width(), 5202);
    assert_eq!(bayer_image.mosaic().height(), 3464);
    bayer_image
        .mosaic()
        .save(utils::output_dir().join("bayerimga.tiff"))
        .into_diagnostic()?;
    clerk::info!("Done saving raw image.");
    Ok(())
}
