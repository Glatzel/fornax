use std::io::Read;
use std::io::Write;

use flate2::Compression as FlateCompression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use fornax_core::RawMosaic;

use crate::encode::encode_image;
use crate::request::{DecodeRequest, EncodeRequest, EncodedChunks};
use crate::samples::{Geometry, chunk, decode_payloads};
use crate::{CodecError, ICodecDecode, ICodecEncode, error::invalid, predictor};

/// TIFF Deflate decoder using zlib framing.
#[derive(Clone, Copy, Debug, Default)]
pub struct DeflateCodec;

impl ICodecDecode for DeflateCodec {
    fn decode(&self, request: &DecodeRequest<'_>) -> Result<RawMosaic, CodecError> {
        let geometry = Geometry::new(
            request.width,
            request.height,
            request.chunk_width,
            request.chunk_height,
            request.chunks.len(),
        )?;
        let mut decoded = Vec::with_capacity(request.chunks.len());
        for (index, range) in request.chunks.iter().copied().enumerate() {
            let encoded = chunk(request.source, range)?;
            let mut payload = inflate(encoded)
                .map_err(|detail| invalid(format!("Deflate chunk {index}: {detail}")))?;
            predictor::reverse(
                &mut payload,
                request.predictor,
                request.chunk_width,
                request.chunk_height,
                request.bits_per_sample,
                request.sample_format,
                request.byte_order,
            )
            .map_err(|error| invalid(format!("Deflate chunk {index}: {error}")))?;
            decoded.push(payload);
        }
        let payloads = decoded.iter().map(Vec::as_slice).collect::<Vec<_>>();
        decode_payloads(
            &geometry,
            request.bits_per_sample,
            request.sample_format,
            request.byte_order,
            &payloads,
        )
    }
}

impl ICodecEncode for DeflateCodec {
    fn encode(&self, request: &EncodeRequest<'_>) -> Result<EncodedChunks, CodecError> {
        let mut encoded = encode_image(
            request.image,
            request.chunk_width,
            request.chunk_height,
            request.byte_order,
        )?;
        let mut chunks = Vec::with_capacity(encoded.chunks.len());
        for chunk in &mut encoded.chunks {
            predictor::apply(
                chunk,
                request.predictor,
                request.chunk_width,
                request.chunk_height,
                encoded.bits_per_sample,
                encoded.sample_format,
                request.byte_order,
            )?;
            chunks.push(deflate(chunk)?);
        }
        Ok(EncodedChunks { chunks })
    }
}

pub(crate) fn inflate(encoded: &[u8]) -> Result<Vec<u8>, String> {
    let mut decoder = ZlibDecoder::new(encoded);
    let mut decoded = Vec::new();
    decoder
        .read_to_end(&mut decoded)
        .map_err(|error| format!("payload is invalid: {error}"))?;
    Ok(decoded)
}

pub(crate) fn deflate(data: &[u8]) -> Result<Vec<u8>, CodecError> {
    let mut encoder = ZlibEncoder::new(Vec::new(), FlateCompression::fast());
    encoder
        .write_all(data)
        .map_err(|error| invalid(format!("Deflate setup failed: {error}")))?;
    encoder
        .finish()
        .map_err(|error| invalid(format!("Deflate encoding failed: {error}")))
}
