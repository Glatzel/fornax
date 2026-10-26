use fornax_core::BayerPattern;
use fornax_decoder_protocol::DecodedRawActive;

fn main() -> mischief::Result<()> {
    fornax_devtool::setup();
    let libraw = libraw::Libraw::default();
    let mosaic: DecodedRawActive<f32> = libraw
        .open_file(&fornax_devtool::raw_file())?
        .unpack()?
        .get_decoded_raw_active()?;
    let bayer_pattern = libraw.bayer_pattern()?;
    clerk::info!("Bayer pattern: {}", bayer_pattern);
    assert_eq!(bayer_pattern, BayerPattern::GBRG);
    assert_eq!(mosaic.width(), 5202);
    assert_eq!(mosaic.height(), 3464);

    mosaic.save(
        fornax_devtool::temp_dir()
            .join("libraw")
            .join("bayerimga.tiff"),
    )?;
    clerk::info!("Done saving raw image.");
    Ok(())
}
#[test]
fn test() -> mischief::Result<()> { main() }
