//! Raw sample codecs shared by Fornax decoder implementations.
//!
//! The crate decodes format-neutral sample layouts into
//! [`fornax_core::RawMosaic`] values. Container metadata mapping remains in
//! each format crate.
//!
//! # Examples
//!
//! ```
//! use fornax_codec::{ByteOrder, Chunk, DecodeRequest, Predictor, SampleFormat};
//!
//! let source = [0_u8, 1, 2, 3];
//! let chunks = [Chunk {
//!     offset: 0,
//!     byte_count: source.len(),
//! }];
//! let request = DecodeRequest {
//!     source: &source,
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
//! assert_eq!(
//!     fornax_codec::decode(fornax_codec::Compression::Uncompressed, &request)
//!         .unwrap()
//!         .dimensions(),
//!     (2, 2)
//! );
//! ```

pub mod deflate;
mod encode;
mod error;
pub mod jpeg;
pub mod jpegxl;
pub mod ljpeg;
pub mod predictor;
mod request;
mod samples;
pub mod uncompressed;
pub mod zstd;

pub use deflate::DeflateCodec;
pub use error::CodecError;
pub use jpeg::LossyJpegCodec;
pub use jpegxl::JpegXlCodec;
pub use ljpeg::LosslessJpegCodec;
pub use predictor::Predictor;
pub use request::{
    ByteOrder, Chunk, Compression, DecodeRequest, EncodeRequest, EncodedChunks, SampleFormat,
};
pub use uncompressed::UncompressedCodec;
pub use zstd::ZstdCodec;

/// Encodes a raw image using the selected compression scheme.
///
/// # Errors
///
/// Returns [`CodecError`] when the selected codec does not support encoding
/// or rejects the sample layout.
///
/// # Examples
///
/// ```
/// use fornax_codec::{ByteOrder, Compression, EncodeRequest, Predictor};
/// use fornax_core::{MonoImage, RawMosaic};
///
/// let image = RawMosaic::U8(MonoImage::from_raw(2, 1, vec![1, 2]).unwrap());
/// let request = EncodeRequest {
///     image: &image,
///     chunk_width: 2,
///     chunk_height: 1,
///     byte_order: ByteOrder::Little,
///     predictor: Predictor::None,
/// };
/// assert_eq!(
///     fornax_codec::encode(Compression::Uncompressed, &request)
///         .unwrap()
///         .chunks,
///     vec![vec![1, 2]]
/// );
/// ```
pub fn encode(
    compression: Compression,
    request: &EncodeRequest<'_>,
) -> Result<EncodedChunks, CodecError> {
    match compression {
        Compression::Uncompressed => UncompressedCodec.encode(request),
        Compression::LosslessJpeg => LosslessJpegCodec.encode(request),
        Compression::Deflate => DeflateCodec.encode(request),
        Compression::Zstd => ZstdCodec::default().encode(request),
        Compression::LossyJpeg | Compression::JpegXl => {
            Err(error::unsupported("the selected codec is decode-only"))
        }
    }
}

/// Decodes an encoded raw image into storage-typed samples.
///
/// # Examples
///
/// ```
/// use fornax_codec::{ByteOrder, Chunk, DecodeRequest, Predictor, SampleFormat};
///
/// fn assert_decoder<T: fornax_codec::ICodecDecode>() {}
/// assert_decoder::<fornax_codec::UncompressedCodec>();
///
/// let source = [7_u8; 4];
/// let chunks = [Chunk {
///     offset: 0,
///     byte_count: source.len(),
/// }];
/// let request = DecodeRequest {
///     source: &source,
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
/// assert_eq!(
///     fornax_codec::decode(fornax_codec::Compression::Uncompressed, &request)
///         .unwrap()
///         .dimensions(),
///     (2, 2)
/// );
/// ```
pub trait ICodecDecode {
    /// Decodes one complete image described by `request`.
    ///
    /// # Errors
    ///
    /// Returns [`CodecError`] when the payload, geometry, or sample layout is
    /// invalid or the codec does not support the requested format.
    fn decode(&self, request: &DecodeRequest<'_>) -> Result<RawMosaic, CodecError>;
}

/// Encodes a raw image into independently stored chunks.
///
/// # Examples
///
/// ```
/// fn assert_encoder<T: fornax_codec::ICodecEncode>() {}
/// assert_encoder::<fornax_codec::DeflateCodec>();
/// ```
pub trait ICodecEncode {
    /// Encodes `request` into row-major chunks.
    ///
    /// # Errors
    ///
    /// Returns [`CodecError`] when the sample type, layout, predictor, or
    /// payload cannot be encoded by this codec.
    fn encode(&self, request: &EncodeRequest<'_>) -> Result<EncodedChunks, CodecError>;
}

use fornax_core::RawMosaic;

/// Decodes a raw image using the selected compression scheme.
///
/// # Errors
///
/// Returns [`CodecError`] when the selected codec rejects the payload or the
/// requested operation is unavailable.
///
/// # Examples
///
/// ```
/// use fornax_codec::{ByteOrder, Chunk, DecodeRequest, Predictor, SampleFormat};
///
/// let source = [42_u8; 4];
/// let chunks = [Chunk {
///     offset: 0,
///     byte_count: source.len(),
/// }];
/// let request = DecodeRequest {
///     source: &source,
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
/// assert_eq!(
///     fornax_codec::decode(fornax_codec::Compression::Uncompressed, &request)
///         .unwrap()
///         .dimensions(),
///     (2, 2)
/// );
/// ```
pub fn decode(
    compression: Compression,
    request: &DecodeRequest<'_>,
) -> Result<RawMosaic, CodecError> {
    match compression {
        Compression::Uncompressed => UncompressedCodec.decode(request),
        Compression::LosslessJpeg => LosslessJpegCodec.decode(request),
        Compression::Deflate => DeflateCodec.decode(request),
        Compression::LossyJpeg => LossyJpegCodec.decode(request),
        Compression::JpegXl => JpegXlCodec.decode(request),
        Compression::Zstd => ZstdCodec::default().decode(request),
    }
}
