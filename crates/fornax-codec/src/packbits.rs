//! TIFF PackBits decoding and encoding.

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
            .map_err(|error| invalid(format!("PackBits chunk {index}: {error}")))?;
        let mut payload = unpack(encoded)
            .map_err(|detail| invalid(format!("PackBits chunk {index}: {detail}")))?;
        predictor::reverse(
            &mut payload,
            request.predictor,
            request.chunk_width,
            request.chunk_height,
            request.bits_per_sample,
            request.sample_format,
            request.byte_order,
        )
        .map_err(|error| invalid(format!("PackBits chunk {index}: {error}")))?;
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
        chunks.push(pack(chunk));
    }
    Ok(EncodedChunks { chunks })
}

fn unpack(encoded: &[u8]) -> Result<Vec<u8>, String> {
    let mut decoded = Vec::new();
    let mut index = 0;
    while index < encoded.len() {
        let control = encoded[index] as i8;
        index += 1;
        match control {
            0..=127 => {
                let count = usize::from(control as u8) + 1;
                let end = index
                    .checked_add(count)
                    .filter(|&end| end <= encoded.len())
                    .ok_or_else(|| "literal run is truncated".to_owned())?;
                decoded.extend_from_slice(&encoded[index..end]);
                index = end;
            }
            -127..=-1 => {
                let value = *encoded
                    .get(index)
                    .ok_or_else(|| "repeat run is truncated".to_owned())?;
                index += 1;
                let count = usize::from((1 - i16::from(control)) as u8);
                decoded.resize(decoded.len() + count, value);
            }
            -128 => {}
        }
    }
    Ok(decoded)
}

fn pack(data: &[u8]) -> Vec<u8> {
    let mut encoded = Vec::new();
    let mut index = 0;
    while index < data.len() {
        let run = repeated_len(data, index);
        if run >= 3 {
            encoded.push((1 - i16::from(u8::try_from(run).unwrap())) as i8 as u8);
            encoded.push(data[index]);
            index += run;
            continue;
        }

        let start = index;
        index += run;
        while index < data.len() {
            let next_run = repeated_len(data, index);
            if next_run >= 3 || index - start + next_run > 128 {
                break;
            }
            index += next_run;
        }
        let literal = &data[start..index];
        encoded.push(u8::try_from(literal.len() - 1).unwrap());
        encoded.extend_from_slice(literal);
    }
    encoded
}

fn repeated_len(data: &[u8], start: usize) -> usize {
    let value = data[start];
    data[start..]
        .iter()
        .take(128)
        .take_while(|&&sample| sample == value)
        .count()
}

#[cfg(test)]
mod tests {
    use crate::{
        ByteOrder, Chunk, DecodeCompression, DecodeRequest, EncodeCompression, EncodeRequest,
        Predictor, SampleBuffer, SampleFormat, decode, encode,
    };

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
            compression: DecodeCompression::PackBits,
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
            compression: EncodeCompression::PackBits,
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
    fn decodes_repeat_literal_and_noop_controls() {
        let source = [0xfe, 0xaa, 0x80, 0x02, 1, 2, 3];
        let chunks = [Chunk {
            offset: 0,
            byte_count: source.len(),
        }];
        let request = decode_request(&source, &chunks, 6, 1, 6, 1, Predictor::None);

        let decoded = decode(&request).unwrap();

        assert_eq!(decoded.into_raw_u8().unwrap(), [0xaa, 0xaa, 0xaa, 1, 2, 3]);
    }

    #[test]
    fn decodes_a_literal_only_run() {
        let source = [0x02, 9, 8, 7];
        let chunks = [Chunk {
            offset: 0,
            byte_count: source.len(),
        }];
        let request = decode_request(&source, &chunks, 3, 1, 3, 1, Predictor::None);

        assert_eq!(decode(&request).unwrap().into_raw_u8().unwrap(), [9, 8, 7]);
    }

    #[test]
    fn packbits_round_trips() {
        let image = SampleBuffer::from_u8(8, 1, vec![0xaa, 0xaa, 0xaa, 1, 2, 2, 2, 3]).unwrap();
        let encoded = encode(&encode_request(&image, 8, 1, Predictor::None)).unwrap();
        let (source, chunks) = source_and_chunks(encoded.chunks);
        let request = decode_request(&source, &chunks, 8, 1, 8, 1, Predictor::None);

        assert_eq!(decode(&request).unwrap(), image);
    }

    #[test]
    fn packbits_reverses_horizontal_predictor_after_decompression() {
        let image = SampleBuffer::from_u8(4, 1, vec![10, 12, 13, 15]).unwrap();
        let encoded = encode(&encode_request(&image, 4, 1, Predictor::HorizontalInteger)).unwrap();
        let (source, chunks) = source_and_chunks(encoded.chunks);
        let request = decode_request(&source, &chunks, 4, 1, 4, 1, Predictor::HorizontalInteger);

        assert_eq!(decode(&request).unwrap(), image);
    }

    #[test]
    fn packbits_preserves_row_major_partial_edge_chunks() {
        let image = SampleBuffer::from_u8(5, 3, (0_u8..15).collect()).unwrap();
        let encoded = encode(&encode_request(&image, 4, 2, Predictor::None)).unwrap();
        assert_eq!(encoded.chunks.len(), 4);
        let (source, chunks) = source_and_chunks(encoded.chunks);
        let request = decode_request(&source, &chunks, 5, 3, 4, 2, Predictor::None);

        assert_eq!(decode(&request).unwrap(), image);
    }

    #[test]
    fn packbits_reports_the_malformed_chunk_index() {
        let source = [0x00, 0x11, 0x7f];
        let chunks = [
            Chunk {
                offset: 0,
                byte_count: 2,
            },
            Chunk {
                offset: 2,
                byte_count: 1,
            },
        ];
        let request = decode_request(&source, &chunks, 130, 1, 128, 1, Predictor::None);

        let error = decode(&request).unwrap_err().to_string();

        assert!(error.contains("PackBits chunk 1"), "{error}");
    }

    #[test]
    fn packbits_reports_truncated_repeat_runs() {
        let source = [0x00, 0x11, 0xfe];
        let chunks = [
            Chunk {
                offset: 0,
                byte_count: 2,
            },
            Chunk {
                offset: 2,
                byte_count: 1,
            },
        ];
        let request = decode_request(&source, &chunks, 130, 1, 128, 1, Predictor::None);

        let error = decode(&request).unwrap_err().to_string();

        assert!(error.contains("PackBits chunk 1"), "{error}");
    }

    #[test]
    fn packbits_round_trips_control_run_boundaries() {
        for length in [1_usize, 2, 3, 127, 128, 129, 255, 256, 257] {
            let width = u32::try_from(length).unwrap();
            let repeated = SampleBuffer::from_u8(width, 1, vec![0x55; length]).unwrap();
            let encoded = encode(&encode_request(&repeated, length, 1, Predictor::None)).unwrap();
            let (source, chunks) = source_and_chunks(encoded.chunks);
            let request = decode_request(&source, &chunks, length, 1, length, 1, Predictor::None);
            assert_eq!(
                decode(&request).unwrap(),
                repeated,
                "repeat length {length}"
            );

            let literals = SampleBuffer::from_u8(
                width,
                1,
                (0..length).map(|value| (value % 251) as u8).collect(),
            )
            .unwrap();
            let encoded = encode(&encode_request(&literals, length, 1, Predictor::None)).unwrap();
            let (source, chunks) = source_and_chunks(encoded.chunks);
            let request = decode_request(&source, &chunks, length, 1, length, 1, Predictor::None);
            assert_eq!(
                decode(&request).unwrap(),
                literals,
                "literal length {length}"
            );
        }
    }
}
