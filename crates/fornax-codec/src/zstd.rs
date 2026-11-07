use fornax_core::RawMosaic;

use crate::encode::encode_image;
use crate::request::{DecodeRequest, EncodeRequest, EncodedChunks};
use crate::samples::{Geometry, chunk, decode_payloads};
use crate::{CodecError, ICodecDecode, ICodecEncode, error::invalid, predictor};

/// TIFF's experimental Zstandard raw chunk decoder.
#[derive(Clone, Copy, Debug, Default)]
pub struct ZstdCodec;

#[cfg(feature = "zstd")]
impl ICodecDecode for ZstdCodec {
    fn decode(&self, request: &DecodeRequest<'_>) -> Result<RawMosaic, CodecError> {
        let geometry = Geometry::new(
            request.width,
            request.height,
            request.chunk_width,
            request.chunk_height,
            request.chunks.len(),
        )?;
        let max_output = geometry
            .chunk_width
            .checked_mul(geometry.chunk_height)
            .and_then(|samples| samples.checked_mul(usize::from(request.bits_per_sample.div_ceil(8))))
            .ok_or_else(|| invalid("Zstandard tile size overflow"))?;
        let mut decoded = Vec::with_capacity(request.chunks.len());
        for (index, range) in request.chunks.iter().copied().enumerate() {
            let encoded = chunk(request.source, range)?;
            let mut payload = decompress(encoded, max_output)
                .map_err(|detail| invalid(format!("Zstandard chunk {index}: {detail}")))?;
            predictor::reverse(
                &mut payload,
                request.predictor,
                request.chunk_width,
                request.chunk_height,
                request.bits_per_sample,
                request.sample_format,
                request.byte_order,
            )
            .map_err(|error| invalid(format!("Zstandard chunk {index}: {error}")))?;
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

#[cfg(feature = "zstd")]
impl ICodecEncode for ZstdCodec {
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
            chunks.push(compress(chunk, 3)?);
        }
        Ok(EncodedChunks { chunks })
    }
}

#[cfg(not(feature = "zstd"))]
impl ICodecEncode for ZstdCodec {
    fn encode(&self, _request: &EncodeRequest<'_>) -> Result<EncodedChunks, CodecError> {
        Err(crate::error::unsupported(
            "Zstandard encoding requires the fornax-codec zstd feature",
        ))
    }
}

#[cfg(not(feature = "zstd"))]
impl ICodecDecode for ZstdCodec {
    fn decode(&self, _request: &DecodeRequest<'_>) -> Result<RawMosaic, CodecError> {
        Err(crate::error::unsupported(
            "Zstandard compression requires the fornax-codec zstd feature",
        ))
    }
}

#[cfg(feature = "zstd")]
pub(crate) fn decompress(encoded: &[u8], max_output: usize) -> Result<Vec<u8>, String> {
    use std::io::Read;

    let decoder = structured_zstd::decoding::StreamingDecoder::new(encoded);
    let limit = u64::try_from(max_output)
        .unwrap_or(u64::MAX)
        .saturating_add(1);
    let mut decoded = Vec::new();
    decoder
        .take(limit)
        .read_to_end(&mut decoded)
        .map_err(|error| format!("payload is invalid: {error}"))?;
    if decoded.len() > max_output {
        return Err("payload exceeds the declared tile size".into());
    }
    Ok(decoded)
}

#[cfg(feature = "zstd")]
pub(crate) fn compress(data: &[u8], level: i32) -> Result<Vec<u8>, CodecError> {
    Ok(structured_zstd::encoding::compress_to_vec(
        data,
        structured_zstd::encoding::CompressionLevel::from_level(level),
    ))
}
