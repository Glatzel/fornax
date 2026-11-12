//! TIFF LZW decoding and encoding.

use weezl::BitOrder;
use weezl::decode::Decoder;
use weezl::encode::Encoder;

use crate::encode::encode_image;
use crate::error::invalid;
use crate::samples::{Geometry, chunk, decode_payloads};
use crate::{CodecError, DecodeRequest, EncodeRequest, EncodedChunks, SampleBuffer, predictor};

pub fn decode(request: &DecodeRequest<'_>) -> Result<SampleBuffer, CodecError> {
    let geometry = Geometry::new(
        request.width,
        request.height,
        request.chunk_width,
        request.chunk_height,
        request.chunks.len(),
    )?;
    let mut decoded = Vec::with_capacity(request.chunks.len());
    for (index, range) in request.chunks.iter().copied().enumerate() {
        let encoded = chunk(request.source, range)
            .map_err(|error| invalid(format!("LZW chunk {index}: {error}")))?;
        let mut payload = unpack_lzw(encoded)
            .map_err(|detail| invalid(format!("LZW chunk {index}: {detail}")))?;
        predictor::reverse(
            &mut payload,
            request.predictor,
            request.chunk_width,
            request.chunk_height,
            request.bits_per_sample,
            request.sample_format,
            request.byte_order,
        )
        .map_err(|error| invalid(format!("LZW chunk {index}: {error}")))?;
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

pub fn encode(request: &EncodeRequest<'_>) -> Result<EncodedChunks, CodecError> {
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
        chunks.push(pack_lzw(chunk)?);
    }
    Ok(EncodedChunks { chunks })
}

fn unpack_lzw(encoded: &[u8]) -> Result<Vec<u8>, String> {
    let mut decoder = Decoder::with_tiff_size_switch(BitOrder::Msb, 8);
    let mut decoded = Vec::new();
    decoder
        .into_vec(&mut decoded)
        .decode_all(encoded)
        .status
        .map_err(|error| error.to_string())?;
    Ok(decoded)
}

fn pack_lzw(data: &[u8]) -> Result<Vec<u8>, CodecError> {
    Encoder::with_tiff_size_switch(BitOrder::Msb, 8)
        .encode(data)
        .map_err(|error| invalid(format!("LZW encoding failed: {error}")))
}

#[cfg(test)]
mod tests {
    use weezl::BitOrder;
    use weezl::decode::Decoder;
    use weezl::encode::Encoder;

    use crate::{
        ByteOrder, Chunk, DecodeCompression, DecodeRequest, EncodeCompression, EncodeRequest,
        ICodecDecode, Predictor, SampleBuffer, SampleFormat, encode,
    };

    const TIFF_LZW_HEX: &[u8] = &[0x80, 0x0b, 0x60, 0x50, 0x22, 0x0c, 0x0c, 0x85, 0x01];
    const TIFF_LZW_DECODED: &[u8] = b"-----A---B";

    fn decode_request<'a>(
        source: &'a [u8],
        chunks: &'a [Chunk],
        width: usize,
        height: usize,
        chunk_width: usize,
        chunk_height: usize,
        predictor: Predictor,
    ) -> DecodeRequest<'a> {
        DecodeRequest {
            source,
            width,
            height,
            chunk_width,
            chunk_height,
            chunks,
            bits_per_sample: 8,
            sample_format: SampleFormat::Integer,
            byte_order: ByteOrder::Little,
            predictor,
        }
    }

    fn encode_request(
        image: &SampleBuffer,
        chunk_width: usize,
        chunk_height: usize,
        predictor: Predictor,
    ) -> EncodeRequest<'_> {
        EncodeRequest {
            image,
            compression: EncodeCompression::Lzw,
            chunk_width,
            chunk_height,
            byte_order: ByteOrder::Little,
            predictor,
        }
    }

    fn source_and_chunks(chunks: Vec<Vec<u8>>) -> (Vec<u8>, Vec<Chunk>) {
        let mut source = Vec::new();
        let mut ranges = Vec::with_capacity(chunks.len());
        for bytes in chunks {
            let offset = source.len();
            source.extend_from_slice(&bytes);
            ranges.push(Chunk {
                offset,
                byte_count: bytes.len(),
            });
        }
        (source, ranges)
    }

    #[test]
    fn decodes_fixed_tiff_lzw_vector() {
        let chunks = [Chunk {
            offset: 0,
            byte_count: TIFF_LZW_HEX.len(),
        }];
        let request = decode_request(TIFF_LZW_HEX, &chunks, 10, 1, 10, 1, Predictor::None);

        let decoded = DecodeCompression::Lzw.decode(&request).unwrap();

        assert_eq!(decoded.into_raw_u8().unwrap(), TIFF_LZW_DECODED);
    }

    #[test]
    fn lzw_round_trips() {
        let image = SampleBuffer::from_u8(10, 1, TIFF_LZW_DECODED.to_vec()).unwrap();
        let encoded = encode(&encode_request(&image, 10, 1, Predictor::None)).unwrap();
        let (source, chunks) = source_and_chunks(encoded.chunks);
        let request = decode_request(&source, &chunks, 10, 1, 10, 1, Predictor::None);

        assert_eq!(DecodeCompression::Lzw.decode(&request).unwrap(), image);
    }

    #[test]
    fn lzw_reverses_horizontal_predictor_after_decompression() {
        let image = SampleBuffer::from_u8(4, 1, vec![10, 12, 13, 15]).unwrap();
        let encoded = encode(&encode_request(&image, 4, 1, Predictor::HorizontalInteger)).unwrap();
        let (source, chunks) = source_and_chunks(encoded.chunks);
        let request = decode_request(&source, &chunks, 4, 1, 4, 1, Predictor::HorizontalInteger);

        assert_eq!(DecodeCompression::Lzw.decode(&request).unwrap(), image);
    }

    #[test]
    fn lzw_preserves_row_major_partial_edge_chunks() {
        let image = SampleBuffer::from_u8(5, 3, (0_u8..15).collect()).unwrap();
        let encoded = encode(&encode_request(&image, 4, 2, Predictor::None)).unwrap();
        assert_eq!(encoded.chunks.len(), 4);
        let (source, chunks) = source_and_chunks(encoded.chunks);
        let request = decode_request(&source, &chunks, 5, 3, 4, 2, Predictor::None);

        assert_eq!(DecodeCompression::Lzw.decode(&request).unwrap(), image);
    }

    #[test]
    fn lzw_decodes_tiff_code_width_transitions() {
        let samples = (0_usize..4_096)
            .map(|value| u8::try_from((value * 37) % 251).unwrap())
            .collect::<Vec<_>>();
        let source = Encoder::with_tiff_size_switch(BitOrder::Msb, 8)
            .encode(&samples)
            .unwrap();
        let chunks = [Chunk {
            offset: 0,
            byte_count: source.len(),
        }];
        let request = decode_request(
            &source,
            &chunks,
            samples.len(),
            1,
            4_096,
            1,
            Predictor::None,
        );

        assert_eq!(
            DecodeCompression::Lzw
                .decode(&request)
                .unwrap()
                .into_raw_u8()
                .unwrap(),
            samples
        );
    }

    #[test]
    fn lzw_encoder_emits_tiff_code_width_transitions() {
        let samples = (0_usize..4_096)
            .map(|value| u8::try_from((value * 53) % 241).unwrap())
            .collect::<Vec<_>>();
        let image = SampleBuffer::from_u8(4_096, 1, samples.clone()).unwrap();
        let encoded = encode(&encode_request(&image, 4_096, 1, Predictor::None)).unwrap();
        let decoded = Decoder::with_tiff_size_switch(BitOrder::Msb, 8)
            .decode(&encoded.chunks[0])
            .unwrap();

        assert_eq!(decoded, samples);
    }

    #[test]
    fn lzw_reports_the_malformed_chunk_index() {
        let mut source = TIFF_LZW_HEX.to_vec();
        source.push(0x80);
        let chunks = [
            Chunk {
                offset: 0,
                byte_count: TIFF_LZW_HEX.len(),
            },
            Chunk {
                offset: TIFF_LZW_HEX.len(),
                byte_count: 1,
            },
        ];
        let request = decode_request(&source, &chunks, 20, 1, 10, 1, Predictor::None);

        let error = DecodeCompression::Lzw
            .decode(&request)
            .unwrap_err()
            .to_string();

        assert!(error.contains("LZW chunk 1"), "{error}");
    }
}
