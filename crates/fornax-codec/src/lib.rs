//! Raw sample codecs shared by Fornax decoder implementations.
//!
//! The crate decodes format-neutral sample layouts into
//! [`fornax_core::RawMosaic`] values. Container metadata mapping remains in
//! each format crate.

mod error;
pub mod deflate;
mod encode;
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
pub fn encode(
    compression: Compression,
    request: &EncodeRequest<'_>,
) -> Result<EncodedChunks, CodecError> {
    match compression {
        Compression::Uncompressed => UncompressedCodec.encode(request),
        Compression::LosslessJpeg => LosslessJpegCodec.encode(request),
        Compression::Deflate => DeflateCodec.encode(request),
        Compression::Zstd => ZstdCodec.encode(request),
        Compression::LossyJpeg | Compression::JpegXl => Err(error::unsupported(
            "the selected codec is decode-only",
        )),
    }
}

/// Decodes an encoded raw image into storage-typed samples.
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
        Compression::Zstd => ZstdCodec.decode(request),
    }
}
