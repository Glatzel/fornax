//! Raw sample codecs shared by Fornax decoder implementations.
//!
//! [`decode`] converts format-neutral sample layouts into
//! [`fornax_core::RawMosaic`] values. [`encode`] converts supported mosaics
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
//! assert_eq!(fornax_codec::decode(&request).unwrap().dimensions(), (2, 2));
//! ```

mod deflate;
mod encode;
mod error;
mod jpeg;
mod jpegxl;
mod ljpeg;
mod predictor;
mod request;
mod samples;
mod uncompressed;
mod zstd;

pub use error::CodecError;
use fornax_core::RawMosaic;
pub use predictor::Predictor;
pub use request::{
    ByteOrder, Chunk, DecodeCompression, DecodeRequest, EncodeCompression, EncodeRequest,
    EncodedChunks, SampleFormat,
};

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
/// assert_eq!(fornax_codec::decode(&request).unwrap().dimensions(), (2, 2));
/// ```
pub fn decode(request: &DecodeRequest<'_>) -> Result<RawMosaic, CodecError> {
    match request.compression {
        DecodeCompression::Uncompressed => uncompressed::decode(request),
        DecodeCompression::LosslessJpeg => ljpeg::decode(request),
        DecodeCompression::Deflate => deflate::decode(request),
        DecodeCompression::LossyJpeg => jpeg::decode(request),
        DecodeCompression::JpegXl => jpegxl::decode(request),
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
/// use fornax_codec::{ByteOrder, EncodeCompression, EncodeRequest, Predictor};
/// use fornax_core::{MonoImage, RawMosaic};
///
/// let image = RawMosaic::U8(MonoImage::from_raw(2, 1, vec![1, 2]).unwrap());
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
        EncodeCompression::LosslessJpeg => ljpeg::encode(request),
        EncodeCompression::Deflate => deflate::encode(request),
        EncodeCompression::Zstd { level } => zstd::encode(request, level),
    }
}
