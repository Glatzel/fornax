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
}
