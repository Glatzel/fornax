use fornax_codec::{
    ByteOrder, Chunk, DecodeRequest, ICodecDecode, Predictor, SampleBuffer, SampleFormat,
};
use fornax_raw_core::error::DecodeError;

use crate::codec::SonyCodec;
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
    let (codec, chunk_width, chunk_height) = match layout.compression {
        CompressionFamily::Uncompressed => {
            (SonyCodec::Uncompressed, layout.width, layout.stored_height)
        }
        CompressionFamily::Arw1 => (SonyCodec::Arw1, layout.width, layout.stored_height),
        CompressionFamily::Arw2 => (
            SonyCodec::Arw2 {
                curve: metadata::curve_knots(raw),
            },
            layout.width,
            layout.stored_height,
        ),
        CompressionFamily::Arw6 => (SonyCodec::Arw6, layout.width, layout.stored_height),
        CompressionFamily::SonyLosslessJpeg => (
            SonyCodec::LosslessJpeg,
            layout
                .tile_width
                .ok_or_else(|| invalid("Sony LJPEG image has no tile width"))?,
            layout
                .tile_height
                .ok_or_else(|| invalid("Sony LJPEG image has no tile length"))?,
        ),
    };
    let bits_per_sample = codec_bits(layout);
    let request = DecodeRequest {
        source,
        width: layout.width,
        height: layout.stored_height,
        chunk_width,
        chunk_height,
        chunks: &chunks,
        bits_per_sample,
        sample_format: SampleFormat::Integer,
        byte_order: ByteOrder::Little,
        predictor: Predictor::None,
    };
    match codec.decode(&request).map_err(codec_error)? {
        SampleBuffer::U16(image) => Ok(image.into_raw()),
        other => Err(invalid(format!(
            "Sony codec returned unsupported sample storage: {other:?}"
        ))),
    }
}

fn codec_error(error: impl std::fmt::Display) -> DecodeError { invalid(error.to_string()) }

fn codec_bits(layout: &RawLayout) -> u16 {
    if layout.compression == CompressionFamily::SonyLosslessJpeg
        || (layout.compression == CompressionFamily::Uncompressed
            && matches!(layout.bits_per_sample, 14 | 16))
    {
        16
    } else {
        layout.bits_per_sample
    }
}

#[cfg(test)]
mod tests {
    use super::codec_bits;
    use crate::container::{CompressionFamily, RawLayout};

    fn layout(compression: CompressionFamily, bits_per_sample: u16) -> RawLayout {
        RawLayout {
            compression,
            width: 64,
            stored_height: 32,
            bits_per_sample,
            offsets: vec![0],
            byte_counts: vec![1],
            tile_width: None,
            tile_height: None,
        }
    }

    #[test]
    fn sony_ljpeg_uses_full_u16_precision_for_historical_container_tags() {
        assert_eq!(
            codec_bits(&layout(CompressionFamily::SonyLosslessJpeg, 12)),
            16
        );
    }

    #[test]
    fn uncompressed_14_and_16_bit_paths_use_u16_word_precision() {
        assert_eq!(codec_bits(&layout(CompressionFamily::Uncompressed, 14)), 16);
        assert_eq!(codec_bits(&layout(CompressionFamily::Uncompressed, 16)), 16);
    }
}
