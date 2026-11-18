use fornax_codec::{Chunk, CodecError, SampleBuffer};

pub struct Geometry {
    pub width: usize,
    pub height: usize,
    pub chunk_width: usize,
    pub chunk_height: usize,
    pub across: usize,
}

impl Geometry {
    pub fn new(
        width: usize,
        height: usize,
        chunk_width: usize,
        chunk_height: usize,
        chunk_count: usize,
    ) -> Result<Self, CodecError> {
        if width == 0 || height == 0 {
            return Err(invalid("image dimensions must not be zero"));
        }
        if chunk_width == 0 || chunk_height == 0 {
            return Err(invalid("chunk dimensions must not be zero"));
        }
        let across = width.div_ceil(chunk_width);
        let down = height.div_ceil(chunk_height);
        let expected = across
            .checked_mul(down)
            .ok_or_else(|| invalid("chunk count overflow"))?;
        if chunk_count != expected {
            return Err(invalid(format!(
                "expected {expected} chunks, found {chunk_count}"
            )));
        }
        Ok(Self {
            width,
            height,
            chunk_width,
            chunk_height,
            across,
        })
    }

    pub fn band_stride(&self) -> Result<usize, CodecError> {
        self.chunk_height
            .checked_mul(self.width)
            .ok_or_else(|| invalid("decoded band size overflow"))
    }

    pub fn chunk_sample_count(&self) -> Result<usize, CodecError> {
        self.chunk_width
            .checked_mul(self.chunk_height)
            .ok_or_else(|| invalid("chunk sample count overflow"))
    }
}

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
