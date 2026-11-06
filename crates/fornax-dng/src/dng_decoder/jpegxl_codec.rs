use jxl_oxide::JxlImage;

pub(super) fn decode_u8(encoded: &[u8], width: usize, height: usize) -> Result<Vec<u8>, String> {
    let image = JxlImage::builder()
        .read(encoded)
        .map_err(|error| format!("JPEG XL header is invalid: {error:?}"))?;
    let frame = image
        .render_frame(0)
        .map_err(|error| format!("JPEG XL payload is invalid: {error:?}"))?;
    let mut stream = frame.stream_no_alpha();
    if stream.width() as usize != width
        || stream.height() as usize != height
        || stream.channels() != 1
    {
        return Err(format!(
            "JPEG XL tile is {}x{} with {} channels, expected {width}x{height} with one",
            stream.width(),
            stream.height(),
            stream.channels()
        ));
    }
    let mut output = vec![0_u8; width * height];
    let written = stream.write_to_buffer(&mut output);
    if written != output.len() {
        return Err(format!(
            "JPEG XL decoded {written} samples, expected {}",
            output.len()
        ));
    }
    Ok(output)
}

pub(super) fn decode_u16(
    encoded: &[u8],
    width: usize,
    height: usize,
    bits: u16,
) -> Result<Vec<u16>, String> {
    let image = JxlImage::builder()
        .read(encoded)
        .map_err(|error| format!("JPEG XL header is invalid: {error:?}"))?;
    let frame = image
        .render_frame(0)
        .map_err(|error| format!("JPEG XL payload is invalid: {error:?}"))?;
    let mut stream = frame.stream_no_alpha();
    if stream.width() as usize != width
        || stream.height() as usize != height
        || stream.channels() != 1
    {
        return Err(format!(
            "JPEG XL tile is {}x{} with {} channels, expected {width}x{height} with one",
            stream.width(),
            stream.height(),
            stream.channels()
        ));
    }
    let mut output = vec![0_u16; width * height];
    let written = stream.write_to_buffer(&mut output);
    if written != output.len() {
        return Err(format!(
            "JPEG XL decoded {written} samples, expected {}",
            output.len()
        ));
    }
    if bits < 16 {
        let shift = 16 - bits;
        for value in &mut output {
            *value >>= shift;
        }
    }
    Ok(output)
}
