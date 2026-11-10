use fornax_codec::{
    ByteOrder, Chunk, DecodeCompression, DecodeRequest, Predictor, SampleBuffer, SampleFormat,
};
use fornax_raw_core::error::DecodeError;

use crate::container::{CompressionFamily, RawLayout, invalid};
use crate::metadata;

pub fn samples(
    source: &[u8],
    layout: &RawLayout,
    raw: &fornax_tiff::Directory,
) -> Result<Vec<u16>, DecodeError> {
    let chunks = layout
        .offsets
        .iter()
        .copied()
        .zip(layout.byte_counts.iter().copied())
        .map(|(offset, byte_count)| Chunk { offset, byte_count })
        .collect::<Vec<_>>();
    let (compression, chunk_width, chunk_height) = match layout.compression {
        CompressionFamily::Uncompressed => (
            DecodeCompression::Uncompressed,
            layout.width,
            layout.stored_height,
        ),
        CompressionFamily::Arw1 => (DecodeCompression::Arw1, layout.width, layout.stored_height),
        CompressionFamily::Arw2 => (
            DecodeCompression::Arw2 {
                curve: metadata::curve_knots(raw),
            },
            layout.width,
            layout.stored_height,
        ),
        CompressionFamily::Arw6 => (DecodeCompression::Arw6, layout.width, layout.stored_height),
        CompressionFamily::SonyLosslessJpeg => (
            DecodeCompression::SonyLosslessJpeg,
            layout
                .tile_width
                .ok_or_else(|| invalid("Sony LJPEG image has no tile width"))?,
            layout
                .tile_height
                .ok_or_else(|| invalid("Sony LJPEG image has no tile length"))?,
        ),
    };
    let codec_bits = if layout.compression == CompressionFamily::Uncompressed
        && matches!(layout.bits_per_sample, 14 | 16)
    {
        16
    } else {
        layout.bits_per_sample
    };
    let request = DecodeRequest {
        source,
        compression,
        width: layout.width,
        height: layout.stored_height,
        chunk_width,
        chunk_height,
        chunks: &chunks,
        bits_per_sample: codec_bits,
        sample_format: SampleFormat::Integer,
        byte_order: ByteOrder::Little,
        predictor: Predictor::None,
    };
    match fornax_codec::decode(&request).map_err(codec_error)? {
        SampleBuffer::U16(image) => Ok(image.into_raw()),
        other => Err(invalid(format!(
            "Sony codec returned unsupported sample storage: {other:?}"
        ))),
    }
}

fn codec_error(error: impl std::fmt::Display) -> DecodeError { invalid(error.to_string()) }
