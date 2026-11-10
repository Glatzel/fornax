//! Directly stored sample decoding and encoding.

use bytemuck::Pod;

use crate::encode::encode_image;
use crate::error::invalid;
use crate::request::{DecodeRequest, EncodeRequest, EncodedChunks};
use crate::samples::{Geometry, chunk, decode_payloads};
use crate::{CodecError, Predictor, SampleBuffer};
pub fn decode(request: &DecodeRequest<'_>) -> Result<SampleBuffer, CodecError> {
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

pub fn encode(request: &EncodeRequest<'_>) -> Result<EncodedChunks, CodecError> {
    if request.predictor != Predictor::None {
        return Err(invalid("uncompressed samples do not support prediction"));
    }
    let width = request.image.width();
    let height = request.image.height();
    if usize::try_from(width).ok() == Some(request.chunk_width)
        && usize::try_from(height).ok() == Some(request.chunk_height)
        && width != 0
        && height != 0
    {
        return Ok(EncodedChunks {
            chunks: vec![serialize_full_image(request.image, request.byte_order)],
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
const fn native_order() -> crate::ByteOrder {
    if cfg!(target_endian = "little") {
        crate::ByteOrder::Little
    } else {
        crate::ByteOrder::Big
    }
}

fn serialize_samples<T>(samples: &[T], byte_order: crate::ByteOrder) -> Vec<u8>
where
    T: Pod,
{
    // One memcpy, no per-element work.
    let mut out: Vec<u8> = bytemuck::cast_slice(samples).to_vec();
    if byte_order != native_order() {
        // Reversing the bytes of each sample is a byte swap for u16/u32/f32
        // alike.
        for sample in out.chunks_exact_mut(std::mem::size_of::<T>()) {
            sample.reverse();
        }
    }
    out
}
fn serialize_full_image(image: &SampleBuffer, byte_order: crate::ByteOrder) -> Vec<u8> {
    match image {
        SampleBuffer::U8(s) => s.as_raw().clone(),
        SampleBuffer::U16(s) => serialize_samples(s.as_raw(), byte_order),
        SampleBuffer::U32(s) => serialize_samples(s.as_raw(), byte_order),
        SampleBuffer::F32(s) => serialize_samples(s.as_raw(), byte_order),
        SampleBuffer::F64(s) => serialize_samples(s.as_raw(), byte_order),
    }
}
#[cfg(test)]
mod tests {
    use super::serialize_full_image;
    use crate::{ByteOrder, SampleBuffer};

    #[test]
    fn full_image_serialization_preserves_sample_types_and_byte_order() {
        let cases = [
            (
                SampleBuffer::from_u8(2, 2, vec![1, 2, 3, 4]).unwrap(),
                ByteOrder::Little,
                vec![1, 2, 3, 4],
            ),
            (
                SampleBuffer::from_u16(2, 2, vec![0x0102, 0x0304, 0xabcd, 0xff00]).unwrap(),
                ByteOrder::Little,
                vec![0x02, 0x01, 0x04, 0x03, 0xcd, 0xab, 0x00, 0xff],
            ),
            (
                SampleBuffer::from_u16(2, 2, vec![0x0102, 0x0304, 0xabcd, 0xff00]).unwrap(),
                ByteOrder::Big,
                vec![0x01, 0x02, 0x03, 0x04, 0xab, 0xcd, 0xff, 0x00],
            ),
            (
                SampleBuffer::from_u32(
                    2,
                    2,
                    vec![0x0102_0304, 0x80ff_0011, 0x1122_3344, 0xdead_beef],
                )
                .unwrap(),
                ByteOrder::Little,
                vec![
                    0x04, 0x03, 0x02, 0x01, 0x11, 0x00, 0xff, 0x80, 0x44, 0x33, 0x22, 0x11, 0xef,
                    0xbe, 0xad, 0xde,
                ],
            ),
            (
                SampleBuffer::from_u32(
                    2,
                    2,
                    vec![0x0102_0304, 0x80ff_0011, 0x1122_3344, 0xdead_beef],
                )
                .unwrap(),
                ByteOrder::Big,
                vec![
                    0x01, 0x02, 0x03, 0x04, 0x80, 0xff, 0x00, 0x11, 0x11, 0x22, 0x33, 0x44, 0xde,
                    0xad, 0xbe, 0xef,
                ],
            ),
            (
                SampleBuffer::from_f32(2, 2, vec![1.0, 0.25, -0.0, f32::from_bits(0x7fc0_1234)])
                    .unwrap(),
                ByteOrder::Little,
                vec![
                    0x00, 0x00, 0x80, 0x3f, 0x00, 0x00, 0x80, 0x3e, 0x00, 0x00, 0x00, 0x80, 0x34,
                    0x12, 0xc0, 0x7f,
                ],
            ),
            (
                SampleBuffer::from_f32(2, 2, vec![1.0, 0.25, -0.0, f32::from_bits(0x7fc0_1234)])
                    .unwrap(),
                ByteOrder::Big,
                vec![
                    0x3f, 0x80, 0x00, 0x00, 0x3e, 0x80, 0x00, 0x00, 0x80, 0x00, 0x00, 0x00, 0x7f,
                    0xc0, 0x12, 0x34,
                ],
            ),
        ];

        for (image, byte_order, expected) in cases {
            assert_eq!(
                serialize_full_image(&image, byte_order),
                expected,
                "{image:?} in {byte_order:?} order"
            );
        }
    }
}
