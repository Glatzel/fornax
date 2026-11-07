//! Raw sample codecs shared by Fornax decoder implementations.
//!
//! The crate decodes format-neutral sample layouts into
//! [`fornax_core::RawMosaic`] values. Container metadata mapping remains in
//! each format crate.

mod error;
pub mod predictor;
mod request;
mod samples;
pub mod uncompressed;

pub use error::CodecError;
pub use predictor::Predictor;
pub use request::{
    ByteOrder, Chunk, Compression, DecodeRequest, EncodeRequest, EncodedChunks, SampleFormat,
};
pub use uncompressed::UncompressedCodec;

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
