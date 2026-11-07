use fornax_core::RawMosaic;

use crate::request::DecodeRequest;
use crate::samples::{Geometry, chunk, decode_payloads};
use crate::{CodecError, ICodecDecode};

/// Decoder and future encoder for directly stored sample chunks.
#[derive(Clone, Copy, Debug, Default)]
pub struct UncompressedCodec;

impl ICodecDecode for UncompressedCodec {
    fn decode(&self, request: &DecodeRequest<'_>) -> Result<RawMosaic, CodecError> {
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

