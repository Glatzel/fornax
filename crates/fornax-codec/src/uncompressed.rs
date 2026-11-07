//! Directly stored sample decoding and encoding.

use fornax_core::RawMosaic;

use crate::encode::encode_image;
use crate::error::invalid;
use crate::request::{DecodeRequest, EncodeRequest, EncodedChunks};
use crate::samples::{Geometry, chunk, decode_payloads};
use crate::{CodecError, ICodecDecode, ICodecEncode, Predictor};

/// Decoder and future encoder for directly stored sample chunks.
///
/// # Examples
///
/// ```
/// let codec = fornax_codec::UncompressedCodec;
/// assert_eq!(codec, fornax_codec::UncompressedCodec);
/// ```
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct UncompressedCodec;

impl ICodecDecode for UncompressedCodec {
    fn decode(&self, request: &DecodeRequest<'_>) -> Result<RawMosaic, CodecError> {
        if request.predictor != Predictor::None {
            return Err(invalid("uncompressed samples do not support prediction"));
        }
        let geometry = Geometry::new(
            request.width,
            request.height,
            request.chunk_width,
            request.chunk_height,
            request.chunks.len(),
        )?;
        let payloads = request
            .chunks
            .iter()
            .copied()
            .map(|range| chunk(request.source, range))
            .collect::<Result<Vec<_>, _>>()?;
        decode_payloads(
            &geometry,
            request.bits_per_sample,
            request.sample_format,
            request.byte_order,
            &payloads,
        )
    }
}

impl ICodecEncode for UncompressedCodec {
    fn encode(&self, request: &EncodeRequest<'_>) -> Result<EncodedChunks, CodecError> {
        if request.predictor != Predictor::None {
            return Err(invalid("uncompressed samples do not support prediction"));
        }
        Ok(EncodedChunks {
            chunks: encode_image(
                request.image,
                request.chunk_width,
                request.chunk_height,
                request.byte_order,
            )?
            .chunks,
        })
    }
}
