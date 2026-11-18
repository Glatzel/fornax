use fornax_codec::{Chunk, CodecError, SampleBuffer};

pub fn chunk(source: &[u8], chunk: Chunk) -> Result<&[u8], CodecError> {
    let end = chunk
        .offset
        .checked_add(chunk.byte_count)
        .ok_or_else(|| invalid("chunk range overflow"))?;
    source
        .get(chunk.offset..end)
        .ok_or_else(|| invalid("chunk offset or byte count is out of bounds"))
}

pub fn buffer_u16(
    width: usize,
    height: usize,
    samples: Vec<u16>,
) -> Result<SampleBuffer, CodecError> {
    let width = u32::try_from(width).map_err(|_| invalid("image width exceeds u32"))?;
    let height = u32::try_from(height).map_err(|_| invalid("image height exceeds u32"))?;
    SampleBuffer::from_u16(width, height, samples).map_err(Into::into)
}

fn invalid<T>(detail: T) -> CodecError
where
    T: Into<Box<str>>,
{
    CodecError::InvalidInput(detail.into())
}
