use std::io::Read;

use flate2::read::ZlibDecoder;
use fornax_core::RawMosaic;

use crate::request::DecodeRequest;
use crate::samples::{Geometry, chunk, decode_payloads};
use crate::{CodecError, ICodecDecode, error::invalid, predictor};

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

pub(crate) fn inflate(encoded: &[u8]) -> Result<Vec<u8>, String> {
    let mut decoder = ZlibDecoder::new(encoded);
    let mut decoded = Vec::new();
    decoder
        .read_to_end(&mut decoded)
        .map_err(|error| format!("payload is invalid: {error}"))?;
    Ok(decoded)
}
