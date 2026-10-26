use fornax_core::BayerPattern;
use fornax_decoder_protocol::{DecodedRawActive, DecodedRawFull};
use image::Luma;

fn main() -> mischief::Result<()> {
    fornax_devtool::setup();
    let libraw = libraw::Libraw::default();

    let decoded_full: DecodedRawFull<f32> = libraw
        .open_file(&fornax_devtool::raw_file())?
        .unpack()?
        .get_decoded_raw_full()?;
    assert_eq!(decoded_full.image().width(), 5360);
    assert_eq!(decoded_full.image().height(), 3516);
    fornax_devtool::save_image::<_, _, Luma<u16>, _>(
        decoded_full.image(),
        &fornax_devtool::temp_dir()
            .join("libraw")
            .join("raw_image_full.tiff"),
        1.0,
    )?;
    
    let decoded_active: DecodedRawActive<f32> = libraw.get_decoded_raw_active()?;
    assert_eq!(decoded_active.image().width(), 5202);
    assert_eq!(decoded_active.image().height(), 3464);
    fornax_devtool::save_image::<_, _, Luma<u16>, _>(
        decoded_active.image(),
        &fornax_devtool::temp_dir()
            .join("libraw")
            .join("raw_image_active.tiff"),
        1.0,
    )?;

    let bayer_pattern = libraw.bayer_pattern()?;
    assert_eq!(bayer_pattern, BayerPattern::GBRG);
    Ok(())
}
#[test]
fn test() -> mischief::Result<()> { main() }
