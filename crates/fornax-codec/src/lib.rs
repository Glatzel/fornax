//! Raw sample codecs shared by Fornax decoder implementations.
//!
//! The crate decodes format-neutral sample layouts into
//! [`fornax_core::RawMosaic`] values and encodes supported mosaics into
//! independently stored chunks. Container metadata mapping remains in each
//! format crate.
//!
//! # Examples
//!
//! ```
//! use fornax_codec::{ByteOrder, Chunk, Codec, DecodeRequest, Predictor, SampleFormat};
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
//!     Codec::Uncompressed.decode(&request).unwrap().dimensions(),
//!     (2, 2)
//! );
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
    ByteOrder, Chunk, Codec, DecodeRequest, EncodeRequest, EncodedChunks, SampleFormat,
};

impl Codec {
    /// Decodes one raw image using this codec.
    ///
    /// # Errors
    ///
    /// Returns [`CodecError`] when the payload, geometry, sample layout, or
    /// selected compression scheme is invalid or unsupported.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax_codec::{ByteOrder, Chunk, Codec, DecodeRequest, Predictor, SampleFormat};
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
    ///     Codec::Uncompressed.decode(&request).unwrap().dimensions(),
    ///     (2, 2)
    /// );
    /// ```
    pub fn decode(&self, request: &DecodeRequest<'_>) -> Result<RawMosaic, CodecError> {
        match *self {
            Self::Uncompressed => uncompressed::decode(request),
            Self::LosslessJpeg => ljpeg::decode(request),
            Self::LossyJpeg => jpeg::decode(request),
            Self::Deflate => deflate::decode(request),
            Self::JpegXl => jpegxl::decode(request),
            Self::Zstd { .. } => zstd::decode(request),
        }
    }

    /// Encodes one raw image using this codec.
    ///
    /// # Errors
    ///
    /// Returns [`CodecError`] when the selected codec is decode-only or the
    /// sample type, geometry, predictor, or payload cannot be encoded.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax_codec::{ByteOrder, Codec, EncodeRequest, Predictor};
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
    ///     Codec::Uncompressed.encode(&request).unwrap().chunks,
    ///     vec![vec![1, 2]]
    /// );
    /// ```
    pub fn encode(&self, request: &EncodeRequest<'_>) -> Result<EncodedChunks, CodecError> {
        match *self {
            Self::Uncompressed => uncompressed::encode(request),
            Self::LosslessJpeg => ljpeg::encode(request),
            Self::Deflate => deflate::encode(request),
            Self::Zstd { level } => zstd::encode(request, level),
            Self::LossyJpeg | Self::JpegXl => {
                Err(error::unsupported("the selected codec is decode-only"))
            }
        }
    }
}
