use fornax_core::RawMosaic;

use crate::request::{ByteOrder, SampleFormat};
use crate::samples::Geometry;
use crate::{CodecError, error::invalid};

pub(crate) struct EncodedSamples {
    pub(crate) chunks: Vec<Vec<u8>>,
    pub(crate) bits_per_sample: u16,
    pub(crate) sample_format: SampleFormat,
}

pub(crate) fn encode_image(
    image: &RawMosaic,
    chunk_width: usize,
    chunk_height: usize,
    byte_order: ByteOrder,
) -> Result<EncodedSamples, CodecError> {
    let (width, height) = image.dimensions();
    let width = usize::try_from(width).expect("u32 fits usize on supported platforms");
    let height = usize::try_from(height).expect("u32 fits usize on supported platforms");
    let across = width.div_ceil(chunk_width.max(1));
    let down = height.div_ceil(chunk_height.max(1));
    let chunk_count = across
        .checked_mul(down)
        .ok_or_else(|| invalid("encoded chunk count overflow"))?;
    let geometry = Geometry::new(width, height, chunk_width, chunk_height, chunk_count)?;

    let (bits_per_sample, sample_format, bytes_per_sample) = match image {
        RawMosaic::U8(_) => (8, SampleFormat::Integer, 1),
        RawMosaic::U16(_) => (16, SampleFormat::Integer, 2),
        RawMosaic::U32(_) => (32, SampleFormat::Integer, 4),
        RawMosaic::F32(_) => (32, SampleFormat::Float, 4),
    };
    let chunk_samples = geometry
        .chunk_width
        .checked_mul(geometry.chunk_height)
        .ok_or_else(|| invalid("encoded chunk sample count overflow"))?;
    let chunk_bytes = chunk_samples
        .checked_mul(bytes_per_sample)
        .ok_or_else(|| invalid("encoded chunk byte count overflow"))?;
    let mut chunks = Vec::with_capacity(chunk_count);

    for tile_y_index in 0..geometry.down {
        let tile_y = tile_y_index * geometry.chunk_height;
        let copy_height = geometry.chunk_height.min(height - tile_y);
        for tile_x_index in 0..geometry.across {
            let tile_x = tile_x_index * geometry.chunk_width;
            let copy_width = geometry.chunk_width.min(width - tile_x);
            let mut chunk = vec![0_u8; chunk_bytes];
            for row in 0..copy_height {
                let target_start = row * geometry.chunk_width * bytes_per_sample;
                match image {
                    RawMosaic::U8(samples) => {
                        let source_start = (tile_y + row) * width + tile_x;
                        chunk[target_start..target_start + copy_width].copy_from_slice(
                            &samples.as_raw()[source_start..source_start + copy_width],
                        );
                    }
                    RawMosaic::U16(samples) => {
                        let source_start = (tile_y + row) * width + tile_x;
                        for (sample, target) in samples.as_raw()
                            [source_start..source_start + copy_width]
                            .iter()
                            .zip(chunk[target_start..].chunks_exact_mut(2))
                        {
                            let bytes = match byte_order {
                                ByteOrder::Little => sample.to_le_bytes(),
                                ByteOrder::Big => sample.to_be_bytes(),
                            };
                            target.copy_from_slice(&bytes);
                        }
                    }
                    RawMosaic::U32(samples) => {
                        let source_start = (tile_y + row) * width + tile_x;
                        for (sample, target) in samples.as_raw()
                            [source_start..source_start + copy_width]
                            .iter()
                            .zip(chunk[target_start..].chunks_exact_mut(4))
                        {
                            let bytes = match byte_order {
                                ByteOrder::Little => sample.to_le_bytes(),
                                ByteOrder::Big => sample.to_be_bytes(),
                            };
                            target.copy_from_slice(&bytes);
                        }
                    }
                    RawMosaic::F32(samples) => {
                        let source_start = (tile_y + row) * width + tile_x;
                        for (sample, target) in samples.as_raw()
                            [source_start..source_start + copy_width]
                            .iter()
                            .zip(chunk[target_start..].chunks_exact_mut(4))
                        {
                            let raw = sample.to_bits();
                            let bytes = match byte_order {
                                ByteOrder::Little => raw.to_le_bytes(),
                                ByteOrder::Big => raw.to_be_bytes(),
                            };
                            target.copy_from_slice(&bytes);
                        }
                    }
                }
            }
            chunks.push(chunk);
        }
    }

    Ok(EncodedSamples {
        chunks,
        bits_per_sample,
        sample_format,
    })
}
