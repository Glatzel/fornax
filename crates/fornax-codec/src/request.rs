use crate::Predictor;
use crate::sample_buffer::SampleBuffer;

/// A format-neutral raw sample decoding algorithm.
///
/// Vendor-specific algorithms implement [`crate::ICodecDecode`] in their vendor
/// crates instead of adding variants here.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DecodeCompression {
    /// Samples are stored directly in the chunk payloads.
    Uncompressed,
    /// TIFF lossless JPEG-92.
    #[cfg(feature = "lossless-jpeg")]
    LosslessJpeg,
    /// TIFF Deflate using zlib framing.
    #[cfg(feature = "deflate")]
    Deflate,
    /// TIFF LZW with MSB-first codes.
    #[cfg(feature = "lzw")]
    Lzw,
    /// TIFF PackBits run-length compression.
    #[cfg(feature = "packbits")]
    PackBits,
    /// TIFF lossy JPEG with one luma component.
    #[cfg(feature = "lossy-jpeg")]
    LossyJpeg,
    /// JPEG XL with one sample channel.
    #[cfg(feature = "jpegxl")]
    JpegXl,
    /// TIFF's experimental Zstandard compression.
    #[cfg(feature = "zstd")]
    Zstd,
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
///
/// The selected codec value supplies the compression algorithm and any
/// algorithm-specific parameters.
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

/// A raw sample encoding algorithm.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EncodeCompression {
    /// Samples are stored directly in the chunk payloads.
    Uncompressed,
    /// TIFF lossless JPEG-92.
    #[cfg(feature = "lossless-jpeg")]
    LosslessJpeg,
    /// TIFF Deflate using zlib framing.
    #[cfg(feature = "deflate")]
    Deflate,
    /// TIFF LZW with MSB-first codes.
    #[cfg(feature = "lzw")]
    Lzw,
    /// TIFF PackBits run-length compression.
    #[cfg(feature = "packbits")]
    PackBits,
    /// TIFF's experimental Zstandard compression with an encoding level.
    #[cfg(feature = "zstd")]
    Zstd {
        /// Compression level passed to the Zstandard encoder.
        level: i32,
    },
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

/// Format-neutral description of one raw image encode.
pub struct EncodeRequest<'a> {
    /// Mosaic to encode.
    pub image: &'a SampleBuffer,
    /// Compression algorithm used to encode the chunk payloads.
    pub compression: EncodeCompression,
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
