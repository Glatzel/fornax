//! Raw sample codecs shared by Fornax decoder implementations.
//!
//! [`ICodecDecode`] converts format-neutral sample layouts into
//! [`SampleBuffer`] values. [`encode`] converts supported sample buffers into
//! independently stored chunks. Container metadata mapping and vendor-specific
//! codecs remain in each format crate.
//!
//! # Examples
//!
//! ```
//! use fornax_codec::{
//!     ByteOrder, Chunk, DecodeCompression, DecodeRequest, ICodecDecode, Predictor, SampleFormat,
//! };
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
//! let decoded = DecodeCompression::Uncompressed.decode(&request).unwrap();
//! assert_eq!((decoded.width(), decoded.height()), (2, 2));
//! ```

#[allow(
    dead_code,
    reason = "encoding helpers are used only by feature-gated codecs"
)]
mod encode;
mod error;
pub mod huffman;
#[allow(
    dead_code,
    reason = "predictor helpers are used only by feature-gated codecs"
)]
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
#[doc(hidden)]
pub mod ljpeg;
#[cfg(feature = "lzw")]
mod lzw;
#[cfg(feature = "packbits")]
mod packbits;
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

/// Decodes one raw image using the codec value's algorithm and parameters.
///
/// Vendor crates implement this trait for their crate-private codec enums.
/// Generic compression values implement it for format-neutral codecs.
///
/// # Errors
///
/// Returns [`CodecError`] when the payload, geometry, sample layout, or
/// selected compression scheme is invalid or unsupported.
pub trait ICodecDecode {
    /// Decodes the chunks described by `request`.
    ///
    /// # Errors
    ///
    /// Returns [`CodecError::InvalidInput`] for malformed input and
    /// [`CodecError::Unsupported`] for a recognized but unsupported layout.
    fn decode(&self, request: &DecodeRequest<'_>) -> Result<SampleBuffer, CodecError>;
}

impl ICodecDecode for DecodeCompression {
    fn decode(&self, request: &DecodeRequest<'_>) -> Result<SampleBuffer, CodecError> {
        match self {
            Self::Uncompressed => uncompressed::decode(request),
            #[cfg(feature = "lossless-jpeg")]
            Self::LosslessJpeg => ljpeg::decode(request),
            #[cfg(feature = "deflate")]
            Self::Deflate => deflate::decode(request),
            #[cfg(feature = "lzw")]
            Self::Lzw => lzw::decode(request),
            #[cfg(feature = "packbits")]
            Self::PackBits => packbits::decode(request),
            #[cfg(feature = "lossy-jpeg")]
            Self::LossyJpeg => jpeg::decode(request),
            #[cfg(feature = "jpegxl")]
            Self::JpegXl => jpegxl::decode(request),
            #[cfg(feature = "zstd")]
            Self::Zstd => zstd::decode(request),
        }
    }
}

/// Encodes one raw image using the request's compression algorithm.
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
        #[cfg(feature = "lzw")]
        EncodeCompression::Lzw => lzw::encode(request),
        #[cfg(feature = "packbits")]
        EncodeCompression::PackBits => packbits::encode(request),
        #[cfg(feature = "zstd")]
        EncodeCompression::Zstd { level } => zstd::encode(request, level),
    }
}
