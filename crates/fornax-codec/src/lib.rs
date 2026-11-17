#![deny(unsafe_code)]

//! Raw sample codecs shared by Fornax decoder implementations.
//!
//! [`ICodecDecode`] converts encoded chunks into [`SampleBuffer`] values.
//! [`ICodecEncode`] converts sample buffers into independently stored chunks.
//! Container metadata mapping and vendor-specific codecs remain in each format
//! crate.
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
mod allocation;
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
pub use samples::{unpack_packed_u16_rows, unpack_u16_word_rows};

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

/// Encodes one raw image using this codec value's algorithm and parameters.
pub trait ICodecEncode {
    /// Encodes the image described by `request`.
    ///
    /// # Errors
    ///
    /// Returns [`CodecError::InvalidInput`] for malformed geometry or sample
    /// layouts and [`CodecError::Unsupported`] for a recognized but unsupported
    /// layout.
    fn encode(&self, request: &EncodeRequest<'_>) -> Result<EncodedChunks, CodecError>;
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

impl ICodecEncode for EncodeCompression {
    fn encode(&self, request: &EncodeRequest<'_>) -> Result<EncodedChunks, CodecError> {
        match self {
            Self::Uncompressed => uncompressed::encode(request),
            #[cfg(feature = "lossless-jpeg")]
            Self::LosslessJpeg => ljpeg::encode(request),
            #[cfg(feature = "deflate")]
            Self::Deflate => deflate::encode(request),
            #[cfg(feature = "lzw")]
            Self::Lzw => lzw::encode(request),
            #[cfg(feature = "packbits")]
            Self::PackBits => packbits::encode(request),
            #[cfg(feature = "zstd")]
            Self::Zstd { level } => zstd::encode(request, *level),
        }
    }
}
