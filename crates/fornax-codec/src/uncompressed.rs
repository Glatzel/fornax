//! Directly stored sample decoding and encoding.

use fornax_core::RawMosaic;

use crate::encode::encode_image;
use crate::error::invalid;
use crate::request::{DecodeRequest, EncodeRequest, EncodedChunks};
use crate::samples::{Geometry, chunk, decode_payloads};
use crate::{CodecError, Predictor};

pub(crate) fn decode(request: &DecodeRequest<'_>) -> Result<RawMosaic, CodecError> {
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

pub(crate) fn encode(request: &EncodeRequest<'_>) -> Result<EncodedChunks, CodecError> {
    if request.predictor != Predictor::None {
        return Err(invalid("uncompressed samples do not support prediction"));
    }
    let (width, height) = request.image.dimensions();
    if usize::try_from(width).ok() == Some(request.chunk_width)
        && usize::try_from(height).ok() == Some(request.chunk_height)
        && width != 0
        && height != 0
    {
        return Ok(EncodedChunks {
            chunks: vec![serialize_full_image(request.image, request.byte_order)?],
        });
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

fn serialize_full_image(
    image: &RawMosaic,
    byte_order: crate::ByteOrder,
) -> Result<Vec<u8>, CodecError> {
    match image {
        RawMosaic::U8(samples) => Ok(samples.as_raw().clone()),
        RawMosaic::U16(samples) => serialize_samples(samples.as_raw(), |sample| match byte_order {
            crate::ByteOrder::Little => sample.to_le_bytes(),
            crate::ByteOrder::Big => sample.to_be_bytes(),
        }),
        RawMosaic::U32(samples) => serialize_samples(samples.as_raw(), |sample| match byte_order {
            crate::ByteOrder::Little => sample.to_le_bytes(),
            crate::ByteOrder::Big => sample.to_be_bytes(),
        }),
        RawMosaic::F32(samples) => serialize_samples(samples.as_raw(), |sample| match byte_order {
            crate::ByteOrder::Little => sample.to_bits().to_le_bytes(),
            crate::ByteOrder::Big => sample.to_bits().to_be_bytes(),
        }),
    }
}

fn serialize_samples<T, const N: usize>(
    samples: &[T],
    to_bytes: impl Fn(T) -> [u8; N],
) -> Result<Vec<u8>, CodecError>
where
    T: Copy,
{
    let capacity = samples
        .len()
        .checked_mul(N)
        .ok_or_else(|| invalid("encoded image byte count overflow"))?;
    let mut encoded: Vec<u8> = Vec::with_capacity(capacity);
    let target = encoded.as_mut_ptr();
    for (index, sample) in samples.iter().copied().enumerate() {
        let bytes = to_bytes(sample);
        let offset = index * N;
        // SAFETY: `capacity` reserves exactly `samples.len() * N` bytes, and
        // each iteration initializes the unique range `offset..offset + N`.
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), target.add(offset), N);
        }
    }
    // SAFETY: every byte in the reserved range was initialized above.
    unsafe { encoded.set_len(capacity) };
    Ok(encoded)
}

#[cfg(test)]
mod tests {
    use fornax_core::{MonoImage, RawMosaic};

    use super::serialize_full_image;
    use crate::ByteOrder;

    #[test]
    fn full_image_serialization_preserves_sample_types_and_byte_order() {
        let cases = [
            (
                RawMosaic::U8(MonoImage::from_raw(2, 2, vec![1, 2, 3, 4]).unwrap()),
                ByteOrder::Little,
                vec![1, 2, 3, 4],
            ),
            (
                RawMosaic::U16(
                    MonoImage::from_raw(2, 2, vec![0x0102, 0x0304, 0xabcd, 0xff00]).unwrap(),
                ),
                ByteOrder::Little,
                vec![0x02, 0x01, 0x04, 0x03, 0xcd, 0xab, 0x00, 0xff],
            ),
            (
                RawMosaic::U16(
                    MonoImage::from_raw(2, 2, vec![0x0102, 0x0304, 0xabcd, 0xff00]).unwrap(),
                ),
                ByteOrder::Big,
                vec![0x01, 0x02, 0x03, 0x04, 0xab, 0xcd, 0xff, 0x00],
            ),
            (
                RawMosaic::U32(
                    MonoImage::from_raw(
                        2,
                        2,
                        vec![0x0102_0304, 0x80ff_0011, 0x1122_3344, 0xdead_beef],
                    )
                    .unwrap(),
                ),
                ByteOrder::Little,
                vec![
                    0x04, 0x03, 0x02, 0x01, 0x11, 0x00, 0xff, 0x80, 0x44, 0x33, 0x22, 0x11, 0xef,
                    0xbe, 0xad, 0xde,
                ],
            ),
            (
                RawMosaic::U32(
                    MonoImage::from_raw(
                        2,
                        2,
                        vec![0x0102_0304, 0x80ff_0011, 0x1122_3344, 0xdead_beef],
                    )
                    .unwrap(),
                ),
                ByteOrder::Big,
                vec![
                    0x01, 0x02, 0x03, 0x04, 0x80, 0xff, 0x00, 0x11, 0x11, 0x22, 0x33, 0x44, 0xde,
                    0xad, 0xbe, 0xef,
                ],
            ),
            (
                RawMosaic::F32(
                    MonoImage::from_raw(2, 2, vec![1.0, 0.25, -0.0, f32::from_bits(0x7fc0_1234)])
                        .unwrap(),
                ),
                ByteOrder::Little,
                vec![
                    0x00, 0x00, 0x80, 0x3f, 0x00, 0x00, 0x80, 0x3e, 0x00, 0x00, 0x00, 0x80, 0x34,
                    0x12, 0xc0, 0x7f,
                ],
            ),
            (
                RawMosaic::F32(
                    MonoImage::from_raw(2, 2, vec![1.0, 0.25, -0.0, f32::from_bits(0x7fc0_1234)])
                        .unwrap(),
                ),
                ByteOrder::Big,
                vec![
                    0x3f, 0x80, 0x00, 0x00, 0x3e, 0x80, 0x00, 0x00, 0x80, 0x00, 0x00, 0x00, 0x7f,
                    0xc0, 0x12, 0x34,
                ],
            ),
        ];

        for (image, byte_order, expected) in cases {
            assert_eq!(
                serialize_full_image(&image, byte_order).unwrap(),
                expected,
                "{image:?} in {byte_order:?} order"
            );
        }
    }
}
