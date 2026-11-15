use std::mem::size_of;

use rayon::prelude::*;

use crate::error::invalid;
use crate::request::{ByteOrder, SampleFormat};
use crate::samples::Geometry;
use crate::{CodecError, SampleBuffer};

const PARALLEL_ENCODE_MIN_BYTES: usize = 1024 * 1024;

pub struct EncodedSamples {
    pub chunks: Vec<Vec<u8>>,
    pub bits_per_sample: u16,
    pub sample_format: SampleFormat,
}

pub fn encode_image(
    image: &SampleBuffer,
    chunk_width: usize,
    chunk_height: usize,
    byte_order: ByteOrder,
) -> Result<EncodedSamples, CodecError> {
    let width = usize::try_from(image.width()).expect("u32 fits usize on supported platforms");
    let height = usize::try_from(image.height()).expect("u32 fits usize on supported platforms");
    let chunk_count = width
        .div_ceil(chunk_width.max(1))
        .checked_mul(height.div_ceil(chunk_height.max(1)))
        .ok_or_else(|| invalid("encoded chunk count overflow"))?;
    let geometry = Geometry::new(width, height, chunk_width, chunk_height, chunk_count)?;

    match image {
        SampleBuffer::U8(samples) => encode_typed(samples.as_raw(), &geometry, byte_order),
        SampleBuffer::U16(samples) => encode_typed(samples.as_raw(), &geometry, byte_order),
        SampleBuffer::U32(samples) => encode_typed(samples.as_raw(), &geometry, byte_order),
        SampleBuffer::F32(samples) => encode_typed(samples.as_raw(), &geometry, byte_order),
        SampleBuffer::F64(samples) => encode_typed(samples.as_raw(), &geometry, byte_order),
    }
}

trait Sample: Copy + Sync {
    const BYTES: usize;
    const BITS: u16;
    const FORMAT: SampleFormat;

    fn write_row(source: &[Self], target: &mut [u8], byte_order: ByteOrder);
}

macro_rules! impl_integer_sample {
    ($ty:ty, $bits:expr, $bytes:literal) => {
        impl Sample for $ty {
            const BYTES: usize = size_of::<Self>();
            const BITS: u16 = $bits;
            const FORMAT: SampleFormat = SampleFormat::Integer;

            fn write_row(source: &[Self], target: &mut [u8], byte_order: ByteOrder) {
                match byte_order {
                    ByteOrder::Little => {
                        for (sample, target) in
                            source.iter().zip(target.as_chunks_mut::<$bytes>().0)
                        {
                            target.copy_from_slice(&sample.to_le_bytes());
                        }
                    }
                    ByteOrder::Big => {
                        for (sample, target) in
                            source.iter().zip(target.as_chunks_mut::<$bytes>().0)
                        {
                            target.copy_from_slice(&sample.to_be_bytes());
                        }
                    }
                }
            }
        }
    };
}

impl_integer_sample!(u8, 8, 1);
impl_integer_sample!(u16, 16, 2);
impl_integer_sample!(u32, 32, 4);

impl Sample for f32 {
    const BYTES: usize = size_of::<Self>();
    const BITS: u16 = 32;
    const FORMAT: SampleFormat = SampleFormat::Float;

    fn write_row(source: &[Self], target: &mut [u8], byte_order: ByteOrder) {
        match byte_order {
            ByteOrder::Little => {
                for (sample, target) in source.iter().zip(target.as_chunks_mut::<4>().0) {
                    target.copy_from_slice(&sample.to_bits().to_le_bytes());
                }
            }
            ByteOrder::Big => {
                for (sample, target) in source.iter().zip(target.as_chunks_mut::<4>().0) {
                    target.copy_from_slice(&sample.to_bits().to_be_bytes());
                }
            }
        }
    }
}

impl Sample for f64 {
    const BYTES: usize = size_of::<Self>();
    const BITS: u16 = 64;
    const FORMAT: SampleFormat = SampleFormat::Float;

    fn write_row(source: &[Self], target: &mut [u8], byte_order: ByteOrder) {
        match byte_order {
            ByteOrder::Little => {
                for (sample, target) in source.iter().zip(target.as_chunks_mut::<8>().0) {
                    target.copy_from_slice(&sample.to_bits().to_le_bytes());
                }
            }
            ByteOrder::Big => {
                for (sample, target) in source.iter().zip(target.as_chunks_mut::<8>().0) {
                    target.copy_from_slice(&sample.to_bits().to_be_bytes());
                }
            }
        }
    }
}

fn encode_typed<S>(
    source: &[S],
    geometry: &Geometry,
    byte_order: ByteOrder,
) -> Result<EncodedSamples, CodecError>
where
    S: Sample,
{
    debug_assert_eq!(
        source.len(),
        geometry.width.checked_mul(geometry.height).unwrap()
    );
    let chunk_samples = geometry.chunk_sample_count()?;
    let chunk_bytes = chunk_samples
        .checked_mul(S::BYTES)
        .ok_or_else(|| invalid("encoded chunk byte count overflow"))?;
    let chunk_count = geometry
        .across
        .checked_mul(geometry.down)
        .ok_or_else(|| invalid("encoded chunk count overflow"))?;
    let target_row_bytes = geometry.chunk_width * S::BYTES;
    let encode_chunk = |chunk_index| {
        let tile_y_index = chunk_index / geometry.across;
        let tile_x_index = chunk_index % geometry.across;
        let tile_y = tile_y_index * geometry.chunk_height;
        let copy_height = geometry.chunk_height.min(geometry.height - tile_y);
        let tile_x = tile_x_index * geometry.chunk_width;
        let copy_width = geometry.chunk_width.min(geometry.width - tile_x);
        let mut chunk = vec![0_u8; chunk_bytes];
        for row in 0..copy_height {
            let target_start = row * target_row_bytes;
            let target_end = target_start + copy_width * S::BYTES;
            let source_start = (tile_y + row) * geometry.width + tile_x;
            S::write_row(
                &source[source_start..source_start + copy_width],
                &mut chunk[target_start..target_end],
                byte_order,
            );
        }
        chunk
    };
    let use_parallel =
        chunk_count > 1 && chunk_bytes >= PARALLEL_ENCODE_MIN_BYTES.div_ceil(chunk_count);
    let chunks = if use_parallel {
        (0..chunk_count).into_par_iter().map(encode_chunk).collect()
    } else {
        (0..chunk_count).map(encode_chunk).collect()
    };

    Ok(EncodedSamples {
        chunks,
        bits_per_sample: S::BITS,
        sample_format: S::FORMAT,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::samples::decode_payloads;

    const SHAPES: [(&str, u32, u32, usize, usize); 4] = [
        ("padded-tiles", 23, 19, 8, 6),
        ("one-column", 1, 13, 4, 5),
        ("one-row", 13, 1, 4, 5),
        ("single-tile", 5, 3, 8, 6),
    ];

    #[test]
    fn uncompressed_encoding_round_trips_and_matches_original_snapshots() {
        let mut snapshot_mismatches = Vec::new();
        for (shape, width, height, chunk_width, chunk_height) in SHAPES {
            for byte_order in [ByteOrder::Little, ByteOrder::Big] {
                for image in sample_images(width, height) {
                    let sample_type = sample_type(&image);
                    let case = format!("{shape}/{sample_type}/{byte_order:?}");
                    let encoded =
                        encode_image(&image, chunk_width, chunk_height, byte_order).unwrap();

                    let actual = payload_hash(&encoded);
                    let expected = expected_snapshot(&case);
                    if actual != expected {
                        snapshot_mismatches.push(format!("{case} => {actual:#018x}"));
                    }

                    let payloads = encoded.chunks.iter().map(Vec::as_slice).collect::<Vec<_>>();
                    let geometry = Geometry::new(
                        usize::try_from(width).unwrap(),
                        usize::try_from(height).unwrap(),
                        chunk_width,
                        chunk_height,
                        payloads.len(),
                    )
                    .unwrap();
                    let decoded = decode_payloads(
                        &geometry,
                        encoded.bits_per_sample,
                        encoded.sample_format,
                        byte_order,
                        &payloads,
                    )
                    .unwrap();
                    assert_sample_buffer_eq(&decoded, &image, &case);
                }
            }
        }
        assert!(
            snapshot_mismatches.is_empty(),
            "encoder snapshots changed:\n{}",
            snapshot_mismatches.join("\n")
        );
    }

    fn sample_images(width: u32, height: u32) -> [SampleBuffer; 5] {
        let count = usize::try_from(width)
            .unwrap()
            .checked_mul(usize::try_from(height).unwrap())
            .unwrap();
        [
            SampleBuffer::from_u8(
                width,
                height,
                (0..count)
                    .map(|index| ((index * 37 + 11) & 0xff) as u8)
                    .collect(),
            )
            .unwrap(),
            SampleBuffer::from_u16(
                width,
                height,
                (0..count)
                    .map(|index| ((index * 977 + 0x1234) & 0xffff) as u16)
                    .collect(),
            )
            .unwrap(),
            SampleBuffer::from_u32(
                width,
                height,
                (0..count)
                    .map(|index| (index as u32).wrapping_mul(0x9e37_79b9) ^ 0xdead_beef)
                    .collect(),
            )
            .unwrap(),
            SampleBuffer::from_f32(
                width,
                height,
                (0..count)
                    .map(|index| f32::from_bits((index as u32).wrapping_mul(0x9e37_79b9)))
                    .collect(),
            )
            .unwrap(),
            SampleBuffer::from_f64(
                width,
                height,
                (0..count)
                    .map(|index| f64::from_bits((index as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15)))
                    .collect(),
            )
            .unwrap(),
        ]
    }

    fn sample_type(image: &SampleBuffer) -> &'static str {
        match image {
            SampleBuffer::U8(_) => "u8",
            SampleBuffer::U16(_) => "u16",
            SampleBuffer::U32(_) => "u32",
            SampleBuffer::F32(_) => "f32",
            SampleBuffer::F64(_) => "f64",
        }
    }

    fn assert_sample_buffer_eq(actual: &SampleBuffer, expected: &SampleBuffer, case: &str) {
        match (actual, expected) {
            (SampleBuffer::U8(actual), SampleBuffer::U8(expected)) => {
                assert_eq!(
                    actual.as_raw(),
                    expected.as_raw(),
                    "roundtrip failed for {case}"
                );
            }
            (SampleBuffer::U16(actual), SampleBuffer::U16(expected)) => {
                assert_eq!(
                    actual.as_raw(),
                    expected.as_raw(),
                    "roundtrip failed for {case}"
                );
            }
            (SampleBuffer::U32(actual), SampleBuffer::U32(expected)) => {
                assert_eq!(
                    actual.as_raw(),
                    expected.as_raw(),
                    "roundtrip failed for {case}"
                );
            }
            (SampleBuffer::F32(actual), SampleBuffer::F32(expected)) => {
                assert_eq!(
                    actual
                        .as_raw()
                        .iter()
                        .map(|value| value.to_bits())
                        .collect::<Vec<_>>(),
                    expected
                        .as_raw()
                        .iter()
                        .map(|value| value.to_bits())
                        .collect::<Vec<_>>(),
                    "roundtrip failed for {case}"
                );
            }
            (SampleBuffer::F64(actual), SampleBuffer::F64(expected)) => {
                assert_eq!(
                    actual
                        .as_raw()
                        .iter()
                        .map(|value| value.to_bits())
                        .collect::<Vec<_>>(),
                    expected
                        .as_raw()
                        .iter()
                        .map(|value| value.to_bits())
                        .collect::<Vec<_>>(),
                    "roundtrip failed for {case}"
                );
            }
            _ => panic!("sample type changed for {case}"),
        }
    }

    fn payload_hash(encoded: &EncodedSamples) -> u64 {
        let mut hash = 0xcbf2_9ce4_8422_2325_u64;
        hash_bytes(&mut hash, encoded.bits_per_sample.to_le_bytes());
        hash_bytes(
            &mut hash,
            [match encoded.sample_format {
                SampleFormat::Integer => 0,
                SampleFormat::Float => 1,
            }],
        );
        hash_bytes(
            &mut hash,
            u64::try_from(encoded.chunks.len()).unwrap().to_le_bytes(),
        );
        for chunk in &encoded.chunks {
            hash_bytes(&mut hash, u64::try_from(chunk.len()).unwrap().to_le_bytes());
            hash_bytes(&mut hash, chunk.iter().copied());
        }
        hash
    }

    fn hash_bytes(hash: &mut u64, bytes: impl IntoIterator<Item = u8>) {
        for byte in bytes {
            *hash ^= u64::from(byte);
            *hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }

    // FNV-1a hashes captured from the pre-refactor encoder.
    fn expected_snapshot(case: &str) -> u64 {
        match case {
            "padded-tiles/u8/Little" | "padded-tiles/u8/Big" => 0x0f87fc636afcdc5e,
            "padded-tiles/u16/Little" => 0xe2c90fd04fa767e9,
            "padded-tiles/u32/Little" => 0x4a228fb6aaff1634,
            "padded-tiles/f32/Little" => 0xf8c21caf933b6e8d,
            "padded-tiles/f64/Little" => 0x3983d027c79160ea,
            "padded-tiles/u16/Big" => 0x24c69594767933c9,
            "padded-tiles/u32/Big" => 0xd7b2888d40f6240e,
            "padded-tiles/f32/Big" => 0x04b185a472739483,
            "padded-tiles/f64/Big" => 0x157656a52f73689e,
            "one-column/u8/Little" | "one-column/u8/Big" => 0x0d175208d90ffef3,
            "one-column/u16/Little" => 0x3d2391e627f9e887,
            "one-column/u32/Little" => 0xb5f918d20e51edff,
            "one-column/f32/Little" => 0x6f0aff235b2378c2,
            "one-column/f64/Little" => 0x7e89ed37014f4337,
            "one-column/u16/Big" => 0xa6dbc8ae690c89ed,
            "one-column/u32/Big" => 0x85bf03030ace32a1,
            "one-column/f32/Big" => 0x2cb3d554e3fef444,
            "one-column/f64/Big" => 0x71e0382815f60eef,
            "one-row/u8/Little" | "one-row/u8/Big" => 0x2049b01e6a395e80,
            "one-row/u16/Little" => 0xd3dc97a5ef5f2fe0,
            "one-row/u32/Little" => 0x2303d5ca237b1c48,
            "one-row/f32/Little" => 0x587a8f0142b9745d,
            "one-row/f64/Little" => 0x8b837b2c5a287170,
            "one-row/u16/Big" => 0xeb5433f7c3e55dc2,
            "one-row/u32/Big" => 0x233b042eabf27626,
            "one-row/f32/Big" => 0xf60cebdd6029dddf,
            "one-row/f64/Big" => 0xb8e1bb3932f51d58,
            "single-tile/u8/Little" | "single-tile/u8/Big" => 0x4c012d611785e9c4,
            "single-tile/u16/Little" => 0x628dab1348362eea,
            "single-tile/u32/Little" => 0x5a7997525d6b9144,
            "single-tile/f32/Little" => 0x9ff69fd667d0d7e5,
            "single-tile/f64/Little" => 0x1ced8406174094a6,
            "single-tile/u16/Big" => 0xf01db74f073a554e,
            "single-tile/u32/Big" => 0xde211e76328e8344,
            "single-tile/f32/Big" => 0xefcf156437ceae49,
            "single-tile/f64/Big" => 0xfd7668e0f1a8169e,
            _ => panic!("missing snapshot for {case}"),
        }
    }
}
