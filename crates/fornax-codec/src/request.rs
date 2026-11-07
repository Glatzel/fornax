use fornax_core::RawMosaic;

use crate::Predictor;

/// A raw compression scheme understood by the codec dispatcher.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Compression {
    /// Samples are stored directly in the chunk payloads.
    Uncompressed,
    /// TIFF lossless JPEG-92.
    LosslessJpeg,
    /// TIFF Deflate using zlib framing.
    Deflate,
    /// TIFF lossy JPEG with one luma component.
    LossyJpeg,
    /// JPEG XL with one sample channel.
    JpegXl,
    /// TIFF's experimental Zstandard compression identifier.
    Zstd,
}

/// The encoded sample's numeric representation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SampleFormat {
    /// Unsigned integer samples.
    Integer,
    /// IEEE-compatible floating-point samples.
    Float,
}

/// Byte order used by direct sample storage.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ByteOrder {
    /// Least-significant byte first.
    Little,
    /// Most-significant byte first.
    Big,
}

/// One encoded strip or tile within [`DecodeRequest::source`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Chunk {
    /// Byte offset from the start of the source.
    pub offset: usize,
    /// Number of encoded bytes in the chunk.
    pub byte_count: usize,
}

/// Format-neutral description of one raw image decode.
pub struct DecodeRequest<'a> {
    /// Complete source buffer containing the chunk payloads.
    pub source: &'a [u8],
    /// Full image width in samples.
    pub width: usize,
    /// Full image height in samples.
    pub height: usize,
    /// Encoded chunk width in samples.
    pub chunk_width: usize,
    /// Encoded chunk height in samples.
    pub chunk_height: usize,
    /// Row-major strip or tile locations.
    pub chunks: &'a [Chunk],
    /// Stored bits per sample.
    pub bits_per_sample: u16,
    /// Stored sample representation.
    pub sample_format: SampleFormat,
    /// Byte order for direct integer and floating-point storage.
    pub byte_order: ByteOrder,
    /// Reversible predictor applied after decompression.
    pub predictor: Predictor,
}

/// Format-neutral description of one raw image encode.
pub struct EncodeRequest<'a> {
    /// Mosaic to encode.
    pub image: &'a RawMosaic,
    /// Output chunk width in samples.
    pub chunk_width: usize,
    /// Output chunk height in samples.
    pub chunk_height: usize,
    /// Byte order for direct integer and floating-point storage.
    pub byte_order: ByteOrder,
    /// Predictor to apply before compression.
    pub predictor: Predictor,
}

/// Independently stored encoded chunks in row-major order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EncodedChunks {
    /// Encoded bytes for each strip or tile.
    pub chunks: Vec<Vec<u8>>,
}

