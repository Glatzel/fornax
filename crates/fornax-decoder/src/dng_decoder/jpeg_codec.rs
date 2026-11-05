use zune_jpeg::JpegDecoder;
use zune_jpeg::zune_core::bytestream::ZCursor;
use zune_jpeg::zune_core::colorspace::ColorSpace;
use zune_jpeg::zune_core::options::DecoderOptions;

pub(super) fn decode_luma(encoded: &[u8]) -> Result<Vec<u8>, String> {
    let options = DecoderOptions::default().jpeg_set_out_colorspace(ColorSpace::Luma);
    let mut decoder = JpegDecoder::new_with_options(ZCursor::new(encoded), options);
    decoder
        .decode_headers()
        .map_err(|error| format!("lossy JPEG header is invalid: {error:?}"))?;
    let info = decoder
        .info()
        .ok_or_else(|| "lossy JPEG has no image information".to_owned())?;
    if info.components != 1 {
        return Err(format!(
            "lossy JPEG has {} components; a Bayer raw tile requires one",
            info.components
        ));
    }
    let expected = usize::from(info.width)
        .checked_mul(usize::from(info.height))
        .ok_or_else(|| "lossy JPEG dimensions overflow".to_owned())?;
    let pixels = decoder
        .decode()
        .map_err(|error| format!("lossy JPEG payload is invalid: {error:?}"))?;
    if pixels.len() != expected {
        return Err(format!(
            "lossy JPEG decoded {} samples, expected {expected}",
            pixels.len()
        ));
    }
    Ok(pixels)
}
