use fornax_codec::{
    ByteOrder, DecodeCompression, DecodeRequest, NikonTreeId, Predictor, SampleBuffer, SampleFormat,
};
use fornax_raw_core::error::DecodeError;
use fornax_raw_core::{MonoImage, RawMosaic};
use fornax_tiff::Directory;

use crate::container::{LayoutKind, RawLayout};

pub fn samples(
    source: &[u8],
    layout: &RawLayout,
    _raw: &Directory,
) -> Result<RawMosaic, DecodeError> {
    let (compression, bits, byte_order) = match &layout.kind {
        LayoutKind::Packed {
            row_stride,
            bits,
            byte_order,
        } => (
            DecodeCompression::NikonPacked {
                row_stride: *row_stride,
                bits: *bits,
                byte_order: *byte_order,
            },
            *bits,
            *byte_order,
        ),
        LayoutKind::UnpackedWords { bits, byte_order } => (
            DecodeCompression::NikonUnpackedWords {
                bits: *bits,
                byte_order: *byte_order,
            },
            *bits,
            *byte_order,
        ),
        LayoutKind::StripedPacked12 => (
            DecodeCompression::NikonStripedPacked12,
            12,
            ByteOrder::Little,
        ),
        LayoutKind::NikonHuffman { bits, init, curve } => (
            DecodeCompression::NikonHuffman {
                bits: *bits,
                tree: match bits {
                    12 => NikonTreeId::Lossless12,
                    14 => NikonTreeId::Lossless14,
                    _ => {
                        return Err(DecodeError::InvalidInput(
                            "Nikon Huffman bit depth is unsupported".into(),
                        ));
                    }
                },
                init: *init,
                curve: curve.clone(),
            },
            *bits,
            ByteOrder::Little,
        ),
    };
    let decoded = fornax_codec::decode(&DecodeRequest {
        source,
        compression,
        width: layout.width,
        height: layout.height,
        chunk_width: layout.width,
        chunk_height: layout.rows_per_strip,
        chunks: &layout.chunks,
        bits_per_sample: u16::from(bits),
        sample_format: SampleFormat::Integer,
        byte_order,
        predictor: Predictor::None,
    })
    .map_err(codec_error)?;
    let SampleBuffer::U16(image) = decoded else {
        return Err(DecodeError::InvalidInput(
            "Nikon codec returned non-u16 samples".into(),
        ));
    };
    MonoImage::from_raw(
        u32::try_from(layout.width)
            .map_err(|_| DecodeError::InvalidInput("Nikon width exceeds u32".into()))?,
        u32::try_from(layout.height)
            .map_err(|_| DecodeError::InvalidInput("Nikon height exceeds u32".into()))?,
        image.into_raw(),
    )
    .map(RawMosaic::U16)
    .ok_or_else(|| DecodeError::InvalidInput("decoded Nikon mosaic dimensions are invalid".into()))
}

fn codec_error(error: fornax_codec::CodecError) -> DecodeError {
    match error {
        fornax_codec::CodecError::InvalidInput(detail) => DecodeError::InvalidInput(detail),
        fornax_codec::CodecError::Unsupported(detail) => DecodeError::Unsupported(detail),
    }
}
