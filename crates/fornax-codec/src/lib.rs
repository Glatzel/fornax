//! Raw sample codecs shared by Fornax decoder implementations.
//!
//! [`decode`] converts format-neutral sample layouts into
//! [`SampleBuffer`] values. [`encode`] converts supported sample buffers
//! into independently stored chunks. Container metadata mapping remains in
//! each format crate.
//!
//! # Examples
//!
//! ```
//! use fornax_codec::{
//!     ByteOrder, Chunk, DecodeCompression, DecodeRequest, Predictor, SampleFormat,
//! };
//!
//! let source = [0_u8, 1, 2, 3];
//! let chunks = [Chunk {
//!     offset: 0,
//!     byte_count: source.len(),
//! }];
//! let request = DecodeRequest {
//!     source: &source,
//!     compression: DecodeCompression::Uncompressed,
//!     width: 2,
//!     height: 2,
//!     chunk_width: 2,
//!     chunk_height: 2,
//!     chunks: &chunks,
//!     bits_per_sample: 8,
//!     sample_format: SampleFormat::Integer,
//!     byte_order: ByteOrder::Little,
//!     predictor: Predictor::None,
//! };
//! let decoded = fornax_codec::decode(&request).unwrap();
//! assert_eq!((decoded.width(), decoded.height()), (2, 2));
//! ```

mod encode;
mod error;
mod predictor;
mod request;
mod sample_buffer;
mod samples;

#[cfg(feature = "deflate")]
mod deflate;
#[cfg(feature = "lossy-jpeg")]
mod jpeg;
#[cfg(feature = "jpegxl")]
mod jpegxl;
#[cfg(feature = "lossless-jpeg")]
mod ljpeg;
mod uncompressed;
#[cfg(feature = "zstd")]
mod zstd;

pub use error::CodecError;
pub use predictor::Predictor;
pub use request::{
    ByteOrder, Chunk, DecodeCompression, DecodeRequest, EncodeCompression, EncodeRequest,
    EncodedChunks, SampleFormat,
};
pub use sample_buffer::{SampleBuffer, SampleBufferError};

/// Decodes one raw image using the request's compression algorithm.
///
/// # Errors
///
/// Returns [`CodecError`] when the payload, geometry, sample layout, or
/// selected compression scheme is invalid or unsupported.
///
/// # Examples
///
/// ```
/// use fornax_codec::{
///     ByteOrder, Chunk, DecodeCompression, DecodeRequest, Predictor, SampleFormat,
/// };
///
/// let source = [7_u8; 4];
/// let chunks = [Chunk {
///     offset: 0,
///     byte_count: source.len(),
/// }];
/// let request = DecodeRequest {
///     source: &source,
///     compression: DecodeCompression::Uncompressed,
///     width: 2,
///     height: 2,
///     chunk_width: 2,
///     chunk_height: 2,
///     chunks: &chunks,
///     bits_per_sample: 8,
///     sample_format: SampleFormat::Integer,
///     byte_order: ByteOrder::Little,
///     predictor: Predictor::None,
/// };
/// let decoded = fornax_codec::decode(&request).unwrap();
/// assert_eq!((decoded.width(), decoded.height()), (2, 2));
/// ```
pub fn decode(request: &DecodeRequest<'_>) -> Result<SampleBuffer, CodecError> {
    match request.compression {
        DecodeCompression::Uncompressed => uncompressed::decode(request),
        #[cfg(feature = "lossless-jpeg")]
        DecodeCompression::LosslessJpeg => ljpeg::decode(request),
        #[cfg(feature = "deflate")]
        DecodeCompression::Deflate => deflate::decode(request),
        #[cfg(feature = "lossy-jpeg")]
        DecodeCompression::LossyJpeg => jpeg::decode(request),
        #[cfg(feature = "jpegxl")]
        DecodeCompression::JpegXl => jpegxl::decode(request),
        #[cfg(feature = "zstd")]
        DecodeCompression::Zstd => zstd::decode(request),
    }
}
/// Encodes one raw image using the request's compression algorithm.
///
/// # Errors
///
/// Returns [`CodecError`] when the sample type, geometry, predictor, or
/// payload cannot be encoded.
///
/// # Examples
///
/// ```
/// use fornax_codec::{ByteOrder, EncodeCompression, EncodeRequest, Predictor, SampleBuffer};
///
/// let image = SampleBuffer::from_u8(2, 1, vec![1, 2]).unwrap();
/// let request = EncodeRequest {
///     image: &image,
///     compression: EncodeCompression::Uncompressed,
///     chunk_width: 2,
///     chunk_height: 1,
///     byte_order: ByteOrder::Little,
///     predictor: Predictor::None,
/// };
/// assert_eq!(
///     fornax_codec::encode(&request).unwrap().chunks,
///     vec![vec![1, 2]]
/// );
/// ```
pub fn encode(request: &EncodeRequest<'_>) -> Result<EncodedChunks, CodecError> {
    match request.compression {
        EncodeCompression::Uncompressed => uncompressed::encode(request),
        #[cfg(feature = "lossless-jpeg")]
        EncodeCompression::LosslessJpeg => ljpeg::encode(request),
        #[cfg(feature = "deflate")]
        EncodeCompression::Deflate => deflate::encode(request),
        #[cfg(feature = "zstd")]
        EncodeCompression::Zstd { level } => zstd::encode(request, level),
    }
}
