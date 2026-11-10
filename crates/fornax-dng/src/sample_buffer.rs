use fornax_codec::SampleBuffer;
use fornax_raw_core::RawMosaic;
use fornax_raw_core::error::DecodeError;

pub fn from_raw_mosaic(mosaic: &RawMosaic) -> SampleBuffer {
    match mosaic {
        RawMosaic::U8(image) => SampleBuffer::U8(image.clone()),
        RawMosaic::U16(image) => SampleBuffer::U16(image.clone()),
        RawMosaic::U32(image) => SampleBuffer::U32(image.clone()),
        RawMosaic::F32(image) => SampleBuffer::F32(image.clone()),
    }
}

pub fn into_raw_mosaic(buffer: SampleBuffer) -> Result<RawMosaic, DecodeError> {
    match buffer {
        SampleBuffer::U8(image) => Ok(RawMosaic::U8(image)),
        SampleBuffer::U16(image) => Ok(RawMosaic::U16(image)),
        SampleBuffer::U32(image) => Ok(RawMosaic::U32(image)),
        SampleBuffer::F32(image) => Ok(RawMosaic::F32(image)),
        _ => Err(DecodeError::Unsupported(
            "codec returned a sample buffer layout unsupported by RawMosaic".into(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use fornax_codec::SampleBuffer;
    use fornax_raw_core::error::DecodeError;
    use fornax_raw_core::{MonoImage, RawMosaic};

    use super::{from_raw_mosaic, into_raw_mosaic};

    #[test]
    fn raw_mosaic_converts_to_every_sample_buffer_type() {
        let cases = [
            RawMosaic::U8(MonoImage::from_raw(2, 1, vec![1, 2]).unwrap()),
            RawMosaic::U16(MonoImage::from_raw(2, 1, vec![1, 2]).unwrap()),
            RawMosaic::U32(MonoImage::from_raw(2, 1, vec![1, 2]).unwrap()),
            RawMosaic::F32(MonoImage::from_raw(2, 1, vec![1.0, -0.0]).unwrap()),
        ];

        for mosaic in cases {
            let buffer = from_raw_mosaic(&mosaic);
            assert_eq!((buffer.width(), buffer.height()), mosaic.dimensions());
            match (buffer, mosaic) {
                (SampleBuffer::U8(buffer), RawMosaic::U8(mosaic)) => assert_eq!(buffer, mosaic),
                (SampleBuffer::U16(buffer), RawMosaic::U16(mosaic)) => assert_eq!(buffer, mosaic),
                (SampleBuffer::U32(buffer), RawMosaic::U32(mosaic)) => assert_eq!(buffer, mosaic),
                (SampleBuffer::F32(buffer), RawMosaic::F32(mosaic)) => assert_eq!(
                    buffer
                        .as_raw()
                        .iter()
                        .map(|value| value.to_bits())
                        .collect::<Vec<_>>(),
                    mosaic
                        .as_raw()
                        .iter()
                        .map(|value| value.to_bits())
                        .collect::<Vec<_>>()
                ),
                (buffer, mosaic) => panic!("{buffer:?} did not preserve {mosaic:?}"),
            }
        }
    }

    #[test]
    fn sample_buffer_converts_to_every_raw_mosaic_type_with_exact_float_bits() {
        let f32_values = vec![1.0, -0.0, f32::from_bits(0x7fc0_1234)];
        let cases = [
            SampleBuffer::from_u8(2, 1, vec![1, 2]).unwrap(),
            SampleBuffer::from_u16(2, 1, vec![1, 2]).unwrap(),
            SampleBuffer::from_u32(2, 1, vec![1, 2]).unwrap(),
            SampleBuffer::from_f32(3, 1, f32_values.clone()).unwrap(),
        ];

        for buffer in cases {
            let mosaic = into_raw_mosaic(buffer).unwrap();
            match mosaic {
                RawMosaic::F32(image) => assert_eq!(
                    image
                        .as_raw()
                        .iter()
                        .map(|value| value.to_bits())
                        .collect::<Vec<_>>(),
                    f32_values
                        .iter()
                        .map(|value| value.to_bits())
                        .collect::<Vec<_>>()
                ),
                RawMosaic::U8(image) => assert_eq!(image.as_raw().as_slice(), &[1, 2]),
                RawMosaic::U16(image) => assert_eq!(image.as_raw().as_slice(), &[1, 2]),
                RawMosaic::U32(image) => assert_eq!(image.as_raw().as_slice(), &[1, 2]),
            }
        }
    }

    #[test]
    fn f64_sample_buffer_is_rejected_by_the_raw_mosaic_boundary() {
        let buffer = SampleBuffer::from_f64(2, 1, vec![1.0, -0.0]).unwrap();

        assert!(matches!(
            into_raw_mosaic(buffer),
            Err(DecodeError::Unsupported(_))
        ));
    }
}
