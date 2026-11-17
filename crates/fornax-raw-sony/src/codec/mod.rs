use fornax_codec::{CodecError, DecodeCompression, DecodeRequest, ICodecDecode, SampleBuffer};
use llvc3::LLVC3_CURVE;

mod arw1;
mod arw2;
mod arw6;
mod ljpeg;
mod llvc3;

#[derive(Clone, Debug)]
pub enum SonyCodec {
    Uncompressed,
    Arw1,
    Arw2 { curve: Option<[u16; 4]> },
    Arw6,
    LosslessJpeg,
}

impl ICodecDecode for SonyCodec {
    fn decode(&self, request: &DecodeRequest<'_>) -> Result<SampleBuffer, CodecError> {
        match self {
            Self::Uncompressed => DecodeCompression::Uncompressed.decode(request),
            Self::Arw1 => decode_arw1(request),
            Self::Arw2 { curve } => decode_arw2(request, *curve),
            Self::Arw6 => decode_arw6(request),
            Self::LosslessJpeg => decode_ljpeg(request),
        }
    }
}

fn decode_arw1(request: &DecodeRequest<'_>) -> Result<SampleBuffer, CodecError> {
    arw1::decode(request)
}

fn decode_arw2(
    request: &DecodeRequest<'_>,
    curve: Option<[u16; 4]>,
) -> Result<SampleBuffer, CodecError> {
    arw2::decode(request, curve)
}

fn decode_arw6(_request: &DecodeRequest<'_>) -> Result<SampleBuffer, CodecError> {
    arw6::decode(_request)
}

fn decode_ljpeg(request: &DecodeRequest<'_>) -> Result<SampleBuffer, CodecError> {
    ljpeg::decode(request)
}

/// Returns the Sony ARW2 white level implied by its tone-curve knots.
///
/// `None` selects the identity curve. Knots use the stored `SonyRawCurve`
/// representation and map 11-bit decoded codes to 14-bit samples.
///
/// # Examples
///
/// ```text
/// assert_eq!(fornax_raw_sony::codec::arw2_white_level(None), 4095);
/// ```
pub fn arw2_white_level(curve: Option<[u16; 4]>) -> u16 { arw2::white_level(curve) }

/// Returns the fixed Sony ARW6 LLVC3 white level.
///
/// # Examples
///
/// ```text
/// assert_eq!(fornax_raw_sony::codec::arw6_white_level(), 39002);
/// ```
pub const fn arw6_white_level() -> u16 { LLVC3_CURVE[4095] }

#[cfg(test)]
mod tests {
    use fornax_codec::{ByteOrder, Chunk, DecodeRequest, Predictor, SampleBuffer, SampleFormat};

    use super::{decode_arw1, decode_arw2, decode_ljpeg};
    use crate::ljpeg::{decode_interleaved, decode_interleaved_rows};

    fn request<'a>(
        source: &'a [u8],
        chunks: &'a [Chunk],
        width: usize,
        height: usize,
    ) -> DecodeRequest<'a> {
        DecodeRequest {
            source,
            width,
            height,
            chunk_width: width,
            chunk_height: height,
            chunks,
            bits_per_sample: 16,
            sample_format: SampleFormat::Integer,
            byte_order: ByteOrder::Little,
            predictor: Predictor::None,
        }
    }

    #[test]
    fn arw1_zero_difference_codes_decode_zero_samples() {
        let source = [0b0110_1101, 0b1011_0000];
        let chunks = [Chunk {
            offset: 0,
            byte_count: source.len(),
        }];
        let request = request(&source, &chunks, 2, 2);

        let SampleBuffer::U16(image) = decode_arw1(&request).unwrap() else {
            panic!("ARW1 samples must be u16");
        };

        assert_eq!(image.dimensions(), (2, 2));
        assert_eq!(image.as_raw().as_slice(), &[0; 4]);
    }

    #[test]
    fn arw2_zero_blocks_decode_through_the_identity_curve() {
        let source = [0_u8; 32];
        let chunks = [Chunk {
            offset: 0,
            byte_count: source.len(),
        }];
        let request = request(&source, &chunks, 32, 1);

        let SampleBuffer::U16(image) = decode_arw2(&request, None).unwrap() else {
            panic!("ARW2 samples must be u16");
        };

        assert_eq!(image.dimensions(), (32, 1));
        assert_eq!(image.as_raw().as_slice(), &[0; 32]);
    }

    #[test]
    fn sony_ljpeg_one_component_keeps_the_tile_shape() {
        let stream = crate::ljpeg::encode_interleaved_for_test(&[0; 4], 2, 2, 1, 12);
        let chunks = [Chunk {
            offset: 0,
            byte_count: stream.len(),
        }];
        let request = request(&stream, &chunks, 2, 2);

        let SampleBuffer::U16(image) = decode_ljpeg(&request).unwrap() else {
            panic!("Sony LJPEG samples must be u16");
        };

        assert_eq!(image.dimensions(), (2, 2));
        assert_eq!(image.as_raw().as_slice(), &[0; 4]);
    }

    #[test]
    fn sony_ljpeg_four_components_expand_to_two_bayer_rows() {
        let samples = (0_u16..(2 * 2 * 4)).collect::<Vec<_>>();
        let stream = crate::ljpeg::encode_interleaved_for_test(&samples, 2, 2, 4, 12);
        let chunks = [Chunk {
            offset: 0,
            byte_count: stream.len(),
        }];
        let request = request(&stream, &chunks, 4, 4);

        let SampleBuffer::U16(image) = decode_ljpeg(&request).unwrap() else {
            panic!("Sony LJPEG samples must be u16");
        };

        assert_eq!(image.dimensions(), (4, 4));
        assert_eq!(
            image.as_raw().as_slice(),
            &[0, 1, 4, 5, 2, 3, 6, 7, 8, 9, 12, 13, 10, 11, 14, 15]
        );
    }

    #[test]
    fn sony_ljpeg_four_component_fast_path_writes_two_512_wide_bayer_rows() {
        let samples = (0_u16..(256 * 4))
            .map(|index| 8_000 + index * 3)
            .collect::<Vec<_>>();
        let stream = crate::ljpeg::encode_interleaved_for_test(&samples, 256, 1, 4, 14);
        let chunks = [Chunk {
            offset: 0,
            byte_count: stream.len(),
        }];
        let request = request(&stream, &chunks, 512, 2);

        let SampleBuffer::U16(image) = decode_ljpeg(&request).unwrap() else {
            panic!("Sony LJPEG samples must be u16");
        };

        let expected = (0..256)
            .flat_map(|column| [samples[column * 4], samples[column * 4 + 1]])
            .chain((0..256).flat_map(|column| [samples[column * 4 + 2], samples[column * 4 + 3]]))
            .collect::<Vec<_>>();
        assert_eq!(image.as_raw().as_slice(), expected);
    }

    #[test]
    fn sony_ljpeg_four_component_fallbacks_match_generic_decode() {
        for predictor in 2..=7 {
            let (_, stream) = synthetic_four_component(5, 3, predictor, 0);
            assert_sony_matches_generic(&stream, 10, 6, predictor, 0);
        }

        for point_transform in [1, 2] {
            let (_, stream) = synthetic_four_component(7, 2, 1, point_transform);
            assert_sony_matches_generic(&stream, 14, 4, 1, point_transform);
        }

        let width = 7;
        let height = 4;
        let samples = (0_u16..(width * height))
            .map(|index| 8_000 + (index * 97) % 4_000)
            .collect::<Vec<_>>();
        let stream = synthetic_jpeg(
            &samples,
            width,
            height,
            1,
            14,
            SyntheticJpegOptions {
                predictor: 1,
                point_transform: 0,
                restart_interval: None,
            },
        );
        let chunks = [Chunk {
            offset: 0,
            byte_count: stream.len(),
        }];
        let request = request(&stream, &chunks, width as usize, height as usize);
        let SampleBuffer::U16(image) = decode_ljpeg(&request).unwrap() else {
            panic!("Sony LJPEG samples must be u16");
        };
        assert_eq!(image.as_raw().as_slice(), samples);
    }

    #[test]
    fn sony_ljpeg_point_transforms_take_the_generic_path() {
        let (_, stream) = synthetic_four_component(7, 2, 1, 1);
        let handled = decode_interleaved_rows(&stream, 16, 14, 4, |_, _| Ok(())).unwrap();
        assert!(!handled);
    }

    #[test]
    fn sony_ljpeg_four_component_random_rows_match_generic_decode() {
        let mut random = 0x1234_5678_u32;
        for (width, height) in [(1, 1), (3, 5), (5, 2), (7, 3), (11, 2), (13, 4)] {
            let samples = (0..width * height * 4)
                .map(|_| {
                    random = random.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                    (random & 0x3fff) as u16
                })
                .collect::<Vec<_>>();
            let stream = synthetic_jpeg(
                &samples,
                width as u16,
                height as u16,
                4,
                14,
                SyntheticJpegOptions {
                    predictor: 1,
                    point_transform: 0,
                    restart_interval: None,
                },
            );
            assert_sony_matches_generic(&stream, (width * 2) as usize, (height * 2) as usize, 1, 0);
        }
    }

    #[test]
    fn sony_ljpeg_restart_markers_are_rejected_consistently() {
        let (_, stream) = synthetic_four_component_with_restart(4, 3, 2);
        let Err(generic) = decode_interleaved(&stream, 16) else {
            panic!("generic decoder accepted a restart-interval stream");
        };
        assert!(
            generic.contains("restart intervals are unsupported"),
            "{generic}"
        );
        let chunks = [Chunk {
            offset: 0,
            byte_count: stream.len(),
        }];
        let request = request(&stream, &chunks, 8, 6);
        assert!(matches!(
            decode_ljpeg(&request),
            Err(fornax_codec::CodecError::Unsupported(message))
                if message.contains("restart intervals are unsupported")
        ));
    }

    #[test]
    fn sony_ljpeg_truncated_and_header_corrupted_streams_error_without_panicking() {
        let (_, stream) = synthetic_four_component(5, 3, 1, 0);
        let truncated = &stream[..2];
        let chunks = [Chunk {
            offset: 0,
            byte_count: truncated.len(),
        }];
        let truncated_request = request(truncated, &chunks, 10, 6);
        assert!(
            decode_ljpeg(&truncated_request).is_err(),
            "truncated stream of {} bytes decoded",
            truncated.len()
        );

        let mut corrupt = stream;
        corrupt[1] ^= 0x7f;
        let chunks = [Chunk {
            offset: 0,
            byte_count: corrupt.len(),
        }];
        let request = request(&corrupt, &chunks, 10, 6);
        assert!(decode_ljpeg(&request).is_err());
    }

    #[test]
    fn sony_ljpeg_truncated_entropy_returns_an_error() {
        let (_, stream) = synthetic_four_component(5, 3, 1, 0);
        let truncated = &stream[..stream.len() / 2];
        let chunks = [Chunk {
            offset: 0,
            byte_count: truncated.len(),
        }];
        let request = request(truncated, &chunks, 10, 6);
        assert!(decode_ljpeg(&request).is_err());
        assert!(decode_interleaved(truncated, 16).is_err());
    }

    #[test]
    fn sony_ljpeg_stream_ending_at_last_mcu_is_valid() {
        let (samples, stream) = synthetic_four_component(5, 3, 1, 0);
        let chunks = [Chunk {
            offset: 0,
            byte_count: stream.len(),
        }];
        let request = request(&stream, &chunks, 10, 6);
        let SampleBuffer::U16(image) = decode_ljpeg(&request).unwrap() else {
            panic!("Sony LJPEG samples must be u16");
        };
        assert_eq!(image.as_raw().as_slice(), bayer_rows(&samples, 20, 3, 10));
    }

    #[test]
    fn sony_ljpeg_copies_partial_edge_tiles() {
        let samples = (0_u16..16).collect::<Vec<_>>();
        let stream = crate::ljpeg::encode_interleaved_for_test(&samples, 4, 4, 1, 12);
        let chunks = [Chunk {
            offset: 0,
            byte_count: stream.len(),
        }];
        let request = DecodeRequest {
            source: &stream,
            width: 3,
            height: 3,
            chunk_width: 4,
            chunk_height: 4,
            chunks: &chunks,
            bits_per_sample: 12,
            sample_format: SampleFormat::Integer,
            byte_order: ByteOrder::Little,
            predictor: Predictor::None,
        };

        let SampleBuffer::U16(image) = decode_ljpeg(&request).unwrap() else {
            panic!("Sony LJPEG samples must be u16");
        };

        assert_eq!(image.as_raw().as_slice(), &[0, 1, 2, 4, 5, 6, 8, 9, 10]);
    }

    #[test]
    fn sony_ljpeg_rejects_wrong_tile_geometry() {
        let stream = crate::ljpeg::encode_interleaved_for_test(&[0; 4], 2, 2, 1, 12);
        let chunks = [Chunk {
            offset: 0,
            byte_count: stream.len(),
        }];
        let request = request(&stream, &chunks, 4, 2);

        assert!(decode_ljpeg(&request).is_err());
    }

    #[derive(Clone, Copy)]
    struct SyntheticJpegOptions {
        predictor: u8,
        point_transform: u8,
        restart_interval: Option<u16>,
    }

    fn synthetic_jpeg(
        samples: &[u16],
        width: u16,
        height: u16,
        components: u8,
        bits: u8,
        options: SyntheticJpegOptions,
    ) -> Vec<u8> {
        let components = usize::from(components);
        let width = usize::from(width);
        let height = usize::from(height);
        assert_eq!(samples.len(), width * height * components);

        let mut encoded = Vec::new();
        encoded.extend_from_slice(&[0xff, 0xd8, 0xff, 0xc3]);
        push_u16(&mut encoded, (8 + components * 3) as u16);
        encoded.push(bits);
        push_u16(&mut encoded, height as u16);
        push_u16(&mut encoded, width as u16);
        encoded.push(components as u8);
        for component in 1..=components {
            encoded.extend_from_slice(&[component as u8, 0x11, 0]);
        }
        if let Some(interval) = options.restart_interval {
            encoded.extend_from_slice(&[0xff, 0xdd, 0, 4]);
            push_u16(&mut encoded, interval);
        }

        encoded.extend_from_slice(&[0xff, 0xc4, 0, 36, 0]);
        encoded.extend_from_slice(&[0, 0, 0, 0, 17, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        encoded.extend(0_u8..=16);

        encoded.extend_from_slice(&[0xff, 0xda]);
        push_u16(&mut encoded, (6 + components * 2) as u16);
        encoded.push(components as u8);
        for component in 1..=components {
            encoded.extend_from_slice(&[component as u8, 0]);
        }
        encoded.extend_from_slice(&[options.predictor, 0, options.point_transform]);

        let modulus = 1_i32 << bits;
        let half = modulus >> 1;
        let initial = 1_u16 << (bits - options.point_transform - 1);
        let point_transform = options.point_transform;
        let row_width = width * components;
        let mut writer = SyntheticBitWriter::new(&mut encoded);
        for row in 0..height {
            for column in 0..width {
                let mcu = row * width + column;
                if let Some(interval) = options.restart_interval
                    && mcu != 0
                    && mcu.is_multiple_of(usize::from(interval))
                {
                    writer.restart((mcu / usize::from(interval) - 1) % 8);
                }
                for component in 0..components {
                    let index = row * row_width + column * components + component;
                    let prediction = synthetic_prediction(
                        samples,
                        row,
                        column,
                        component,
                        components,
                        width,
                        options.predictor,
                        initial,
                        point_transform,
                    );
                    let sample = samples[index] >> point_transform;
                    let raw = i32::from(sample) - i32::from(prediction);
                    let difference = (raw + half).rem_euclid(modulus) - half;
                    let category = if difference == 0 {
                        0
                    } else {
                        difference.unsigned_abs().ilog2() + 1
                    };
                    writer.write(category, 5);
                    if category != 0 && category != 16 {
                        let value = if difference < 0 {
                            difference + ((1_i32 << category) - 1)
                        } else {
                            difference
                        };
                        writer.write(value as u32, category as u8);
                    }
                }
            }
        }
        writer.finish();
        encoded.extend_from_slice(&[0xff, 0xd9]);
        encoded
    }

    fn synthetic_prediction(
        samples: &[u16],
        row: usize,
        column: usize,
        component: usize,
        components: usize,
        image_width: usize,
        predictor: u8,
        initial: u16,
        point_transform: u8,
    ) -> u16 {
        if row == 0 && column == 0 {
            return initial;
        }
        let row_width = image_width * components;
        if column == 0 {
            return samples[(row - 1) * row_width + component] >> point_transform;
        }
        let left = i32::from(
            samples[row * row_width + (column - 1) * components + component] >> point_transform,
        );
        if row == 0 {
            return left as u16;
        }
        let above = i32::from(
            samples[(row - 1) * row_width + column * components + component] >> point_transform,
        );
        let above_left = i32::from(
            samples[(row - 1) * row_width + (column - 1) * components + component]
                >> point_transform,
        );
        let prediction = match predictor {
            1 => left,
            2 => above,
            3 => above_left,
            4 => left + above - above_left,
            5 => left + ((above - above_left) >> 1),
            6 => above + ((left - above_left) >> 1),
            7 => (left + above) >> 1,
            _ => unreachable!("test predictor is validated by the caller"),
        };
        prediction as u16
    }

    struct SyntheticBitWriter<'a> {
        encoded: &'a mut Vec<u8>,
        buffer: u64,
        used: u8,
    }

    impl<'a> SyntheticBitWriter<'a> {
        const fn new(encoded: &'a mut Vec<u8>) -> Self {
            Self {
                encoded,
                buffer: 0,
                used: 0,
            }
        }

        fn write(&mut self, value: u32, bits: u8) {
            if bits == 0 {
                return;
            }
            self.buffer = (self.buffer << bits) | (u64::from(value) & ((1_u64 << bits) - 1));
            self.used += bits;
            while self.used >= 8 {
                self.used -= 8;
                let byte = (self.buffer >> self.used) as u8;
                self.encoded.push(byte);
                if byte == 0xff {
                    self.encoded.push(0);
                }
            }
            self.buffer &= if self.used == 0 {
                0
            } else {
                (1_u64 << self.used) - 1
            };
        }

        fn finish(&mut self) {
            while self.used != 0 {
                self.write(1, 1);
            }
        }

        fn restart(&mut self, index: usize) {
            self.finish();
            self.encoded.push(0xff);
            self.encoded
                .push(0xd0 + u8::try_from(index % 8).expect("restart index fits u8"));
        }
    }

    fn push_u16(output: &mut Vec<u8>, value: u16) {
        output.extend_from_slice(&value.to_be_bytes());
    }

    fn synthetic_four_component(
        width: usize,
        height: usize,
        predictor: u8,
        point_transform: u8,
    ) -> (Vec<u16>, Vec<u8>) {
        let step = 1_u32 << point_transform;
        let samples = (0..width * height * 4)
            .map(|index| (((8_000 + (index as u32 * 97) % 4_000) / step) * step) as u16)
            .collect::<Vec<_>>();
        let stream = synthetic_jpeg(
            &samples,
            width as u16,
            height as u16,
            4,
            14,
            SyntheticJpegOptions {
                predictor,
                point_transform,
                restart_interval: None,
            },
        );
        (samples, stream)
    }

    fn synthetic_four_component_with_restart(
        width: usize,
        height: usize,
        restart_interval: u16,
    ) -> (Vec<u16>, Vec<u8>) {
        let samples = (0_u16..(width * height * 4) as u16)
            .map(|index| 8_000 + index)
            .collect::<Vec<_>>();
        let stream = synthetic_jpeg(
            &samples,
            width as u16,
            height as u16,
            4,
            14,
            SyntheticJpegOptions {
                predictor: 1,
                point_transform: 0,
                restart_interval: Some(restart_interval),
            },
        );
        (samples, stream)
    }

    fn assert_sony_matches_generic(
        stream: &[u8],
        output_width: usize,
        output_height: usize,
        predictor: u8,
        point_transform: u8,
    ) {
        let decoded = decode_interleaved(stream, 16).unwrap();
        let expected = if decoded.components == 1 {
            decoded.samples
        } else {
            bayer_rows(
                &decoded.samples,
                decoded.width,
                decoded.height,
                output_width,
            )
        };
        let chunks = [Chunk {
            offset: 0,
            byte_count: stream.len(),
        }];
        let request = request(stream, &chunks, output_width, output_height);
        let SampleBuffer::U16(image) = decode_ljpeg(&request).unwrap() else {
            panic!("Sony LJPEG samples must be u16");
        };
        assert_eq!(
            image.as_raw().as_slice(),
            expected,
            "predictor {predictor}, point transform {point_transform}"
        );
    }

    fn bayer_rows(
        samples: &[u16],
        row_width: usize,
        height: usize,
        output_width: usize,
    ) -> Vec<u16> {
        let mut output = vec![0_u16; height * 2 * output_width];
        for row in 0..height {
            for column in 0..output_width / 2 {
                let pixel = &samples[row * row_width + column * 4..][..4];
                let first = row * 2 * output_width + column * 2;
                let second = first + output_width;
                output[first] = pixel[0];
                output[first + 1] = pixel[1];
                output[second] = pixel[2];
                output[second + 1] = pixel[3];
            }
        }
        output
    }
}
