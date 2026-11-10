use crate::Predictor;
use crate::sample_buffer::SampleBuffer;

/// A raw sample decoding algorithm.
///
/// # Examples
///
/// ```
/// let compression = fornax_codec::DecodeCompression::Uncompressed;
/// assert_eq!(compression, fornax_codec::DecodeCompression::Uncompressed);
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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

/// A raw sample encoding algorithm.
///
/// # Examples
///
/// ```
/// let compression = fornax_codec::EncodeCompression::Uncompressed;
/// assert_eq!(compression, fornax_codec::EncodeCompression::Uncompressed);
/// ```
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
///
/// # Examples
///
/// ```
/// assert_ne!(
///     fornax_codec::SampleFormat::Integer,
///     fornax_codec::SampleFormat::Float
/// );
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SampleFormat {
    /// Unsigned integer samples.
    Integer,
    /// IEEE-compatible floating-point samples.
    Float,
}

/// Byte order used by direct sample storage.
///
/// # Examples
///
/// ```
/// assert_ne!(
///     fornax_codec::ByteOrder::Little,
///     fornax_codec::ByteOrder::Big
/// );
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ByteOrder {
    /// Least-significant byte first.
    Little,
    /// Most-significant byte first.
    Big,
}

/// One encoded strip or tile within [`DecodeRequest::source`].
///
/// # Examples
///
/// ```
/// let chunk = fornax_codec::Chunk {
///     offset: 8,
///     byte_count: 4,
/// };
/// assert_eq!(chunk.offset + chunk.byte_count, 12);
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Chunk {
    /// Byte offset from the start of the source.
    pub offset: usize,
    /// Number of encoded bytes in the chunk.
    pub byte_count: usize,
}

/// Format-neutral description of one raw image decode.
///
/// # Examples
///
/// ```
/// let request = fornax_codec::DecodeRequest {
///     source: &[],
///     compression: fornax_codec::DecodeCompression::Uncompressed,
///     width: 1,
///     height: 1,
///     chunk_width: 1,
///     chunk_height: 1,
///     chunks: &[],
///     bits_per_sample: 8,
///     sample_format: fornax_codec::SampleFormat::Integer,
///     byte_order: fornax_codec::ByteOrder::Little,
///     predictor: fornax_codec::Predictor::None,
/// };
/// assert_eq!((request.width, request.height), (1, 1));
/// ```
pub struct DecodeRequest<'a> {
    /// Complete source buffer containing the chunk payloads.
    pub source: &'a [u8],
    /// Compression algorithm used to decode the chunk payloads.
    pub compression: DecodeCompression,
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
///
/// # Examples
///
/// ```
/// use fornax_codec::{EncodeCompression, SampleBuffer};
///
/// let image = SampleBuffer::from_u8(1, 1, vec![0]).unwrap();
/// let request = fornax_codec::EncodeRequest {
///     image: &image,
///     compression: EncodeCompression::Uncompressed,
///     chunk_width: 1,
///     chunk_height: 1,
///     byte_order: fornax_codec::ByteOrder::Little,
///     predictor: fornax_codec::Predictor::None,
/// };
/// assert_eq!((request.chunk_width, request.chunk_height), (1, 1));
/// ```
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
///
/// # Examples
///
/// ```
/// let encoded = fornax_codec::EncodedChunks {
///     chunks: vec![vec![1, 2]],
/// };
/// assert_eq!(encoded.chunks[0], [1, 2]);
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EncodedChunks {
    /// Encoded bytes for each strip or tile.
    pub chunks: Vec<Vec<u8>>,
}
