use std::sync::Arc;

use fornax_codec::{ByteOrder as CodecByteOrder, Chunk};
use fornax_raw_core::error::DecodeError;
use fornax_tiff::constants::{PhotometricInterpretation, TiffTag};
use fornax_tiff::{
    ByteOrder, Directory, ParsedTiff, Tag, Value, unsigned_scalar,
    unsigned_vec as tiff_unsigned_vec,
};

use crate::makernote::MakerNote;

#[derive(Clone, Debug)]
pub enum LayoutKind {
    Packed {
        row_stride: usize,
        bits: u8,
        byte_order: CodecByteOrder,
    },
    UnpackedWords {
        bits: u8,
        byte_order: CodecByteOrder,
    },
    StripedPacked12,
    NikonHuffman {
        bits: u8,
        init: [[u16; 2]; 2],
        curve: Arc<[u16]>,
    },
}

#[derive(Clone, Debug)]
pub struct RawLayout {
    pub width: usize,
    pub height: usize,
    pub rows_per_strip: usize,
    pub chunks: Vec<Chunk>,
    pub kind: LayoutKind,
}

impl RawLayout {
    pub fn parse(parsed: &ParsedTiff, maker: &MakerNote) -> Result<(usize, Self), DecodeError> {
        let (raw_ifd, raw) = select_raw_ifd(parsed)?;
        let width = required_usize(raw, TiffTag::ImageWidth)?;
        let height = required_usize(raw, TiffTag::ImageLength)?;
        if width == 0 || height == 0 {
            return Err(unsupported("Nikon raw dimensions must not be zero"));
        }
        let bits = required_u16(raw, TiffTag::BitsPerSample)?;
        if !matches!(bits, 12 | 14) {
            return Err(unsupported(format!(
                "Nikon BitsPerSample {bits} is unsupported"
            )));
        }
        let samples = optional_u16(raw, TiffTag::SamplesPerPixel).unwrap_or(1);
        if samples != 1 {
            return Err(unsupported(format!(
                "Nikon image has {samples} samples per pixel; one CFA sample is required"
            )));
        }
        let planar = optional_u16(raw, TiffTag::PlanarConfiguration).unwrap_or(1);
        if planar != 1 {
            return Err(unsupported("planar Nikon CFA storage is unsupported"));
        }
        validate_cfa(raw)?;
        if raw.contains_key(&Tag::from(TiffTag::TileOffsets)) {
            return Err(unsupported("tiled Nikon raw images are unsupported"));
        }
        let offsets = unsigned_vec(raw, TiffTag::StripOffsets)?;
        let byte_counts = unsigned_vec(raw, TiffTag::StripByteCounts)?;
        let rows_per_strip = optional_usize(raw, TiffTag::RowsPerStrip).unwrap_or(height);
        if rows_per_strip == 0 {
            return Err(invalid("Nikon RowsPerStrip must not be zero"));
        }
        let expected_strips = height.div_ceil(rows_per_strip);
        if offsets.len() != expected_strips || byte_counts.len() != expected_strips {
            return Err(invalid(format!(
                "Nikon image requires {expected_strips} strips, found {} offsets and {} byte counts",
                offsets.len(),
                byte_counts.len()
            )));
        }
        let chunks = offsets
            .into_iter()
            .zip(byte_counts)
            .map(|(offset, byte_count)| Chunk { offset, byte_count })
            .collect::<Vec<_>>();
        let total_bytes = chunks.iter().try_fold(0_usize, |total, chunk| {
            total
                .checked_add(chunk.byte_count)
                .ok_or_else(|| invalid("Nikon strip byte count total overflow"))
        })?;
        let compression = required_u16(raw, TiffTag::Compression)?;
        let mode = maker.compression()?;
        let byte_order = codec_order(maker.byte_order());
        let kind = if compression == 34_713 && mode == Some(3) {
            if chunks.len() != 1 {
                return Err(unsupported("multi-strip Nikon mode 3 is unsupported"));
            }
            let curve = crate::curve::parse(maker.directory(), bits as u8, maker.byte_order())?;
            LayoutKind::NikonHuffman {
                bits: bits as u8,
                init: curve.init,
                curve: curve.curve,
            }
        } else {
            dispatch(
                compression,
                mode,
                bits,
                width,
                height,
                total_bytes,
                byte_order,
            )?
        };
        validate_strip_sizes(&chunks, &kind, width, height, rows_per_strip)?;
        Ok((
            raw_ifd,
            Self {
                width,
                height,
                rows_per_strip,
                chunks,
                kind,
            },
        ))
    }
}

pub fn validate_make(parsed: &ParsedTiff) -> Result<(), DecodeError> {
    let make = parsed
        .root()
        .get(&Tag::from(TiffTag::Make))
        .and_then(|value| match value {
            Value::Ascii(value) => Some(value.trim()),
            _ => None,
        })
        .unwrap_or_default();
    if make.eq_ignore_ascii_case("Nikon")
        || make.eq_ignore_ascii_case("NIKON CORPORATION")
        || make.eq_ignore_ascii_case("NIKON")
    {
        Ok(())
    } else {
        Err(unsupported(format!(
            "NEF container is not a Nikon image (Make={make:?})"
        )))
    }
}

fn select_raw_ifd(parsed: &ParsedTiff) -> Result<(usize, &Directory), DecodeError> {
    parsed
        .directories
        .iter()
        .enumerate()
        .filter(|(_, directory)| is_raw_ifd(directory))
        .max_by_key(|(_, directory)| {
            optional_usize(directory, TiffTag::ImageWidth)
                .unwrap_or(0)
                .saturating_mul(optional_usize(directory, TiffTag::ImageLength).unwrap_or(0))
        })
        .ok_or_else(|| unsupported("no full-resolution Nikon CFA image IFD was found"))
}

fn is_raw_ifd(directory: &Directory) -> bool {
    let reduced = unsigned_scalar(directory.get(&Tag::from(TiffTag::NewSubfileType)))
        .is_some_and(|value| value & 1 != 0);
    let photometric =
        unsigned_scalar(directory.get(&Tag::from(TiffTag::PhotometricInterpretation)))
            .and_then(|value| u16::try_from(value).ok())
            .is_some_and(|value| value == u16::from(PhotometricInterpretation::Cfa));
    !reduced
        && photometric
        && directory.contains_key(&Tag::from(TiffTag::StripOffsets))
        && directory.contains_key(&Tag::from(TiffTag::StripByteCounts))
}

fn validate_cfa(raw: &Directory) -> Result<(), DecodeError> {
    if !raw.contains_key(&Tag::from(TiffTag::CfaRepeatPatternDim))
        || !raw.contains_key(&Tag::from(TiffTag::CfaPattern))
    {
        return Err(unsupported("Nikon CFA IFD is missing CFA pattern tags"));
    }
    let repeat = unsigned_vec(raw, TiffTag::CfaRepeatPatternDim)?;
    if repeat != [2, 2] {
        return Err(unsupported(format!(
            "Nikon CFA repeat pattern {repeat:?} is not conventional 2x2 Bayer"
        )));
    }
    let pattern = unsigned_vec(raw, TiffTag::CfaPattern)?;
    if pattern.len() != 4
        || pattern.iter().filter(|value| **value == 1).count() != 2
        || !pattern.contains(&0)
        || !pattern.contains(&2)
    {
        return Err(unsupported(format!(
            "Nikon CFA pattern {pattern:?} is not RGB Bayer"
        )));
    }
    Ok(())
}

fn dispatch(
    compression: u16,
    mode: Option<u16>,
    bits: u16,
    width: usize,
    height: usize,
    total_bytes: usize,
    byte_order: CodecByteOrder,
) -> Result<LayoutKind, DecodeError> {
    let pixels = width
        .checked_mul(height)
        .ok_or_else(|| invalid("Nikon sample count overflow"))?;
    let word_bytes = pixels
        .checked_mul(2)
        .ok_or_else(|| invalid("Nikon word sample size overflow"))?;
    let packed_row = width
        .checked_mul(usize::from(bits))
        .map(|value| value.div_ceil(8))
        .ok_or_else(|| invalid("Nikon packed row size overflow"))?;

    if compression == 1 {
        if mode.is_some_and(|mode| mode != 2) {
            return Err(unsupported(format!(
                "TIFF compression 1 conflicts with Nikon mode {mode:?}"
            )));
        }
        if total_bytes != word_bytes {
            return Err(unsupported(
                "TIFF-compressed NEF does not contain 16-bit words",
            ));
        }
        return Ok(LayoutKind::UnpackedWords {
            bits: bits as u8,
            byte_order,
        });
    }
    if compression != 34_713 {
        return Err(unsupported(format!(
            "Nikon TIFF compression {compression} is unsupported"
        )));
    }
    let mode =
        mode.ok_or_else(|| unsupported("Nikon compression 34713 has no 0x0093/0x0051 mode"))?;
    let packed = |required_padding_zero: bool| {
        if !total_bytes.is_multiple_of(height) {
            return Err(unsupported(
                "Nikon packed byte count is not divisible by image height",
            ));
        }
        let row_stride = total_bytes / height;
        if row_stride < packed_row || (required_padding_zero && row_stride != packed_row) {
            return Err(unsupported(format!(
                "Nikon mode {mode} row stride {row_stride} does not match packed width {packed_row}"
            )));
        }
        Ok(LayoutKind::Packed {
            row_stride,
            bits: bits as u8,
            byte_order,
        })
    };
    let words = || {
        if total_bytes == word_bytes {
            Ok(LayoutKind::UnpackedWords {
                bits: bits as u8,
                byte_order,
            })
        } else {
            Err(unsupported(format!(
                "Nikon mode {mode} byte count does not match 16-bit words"
            )))
        }
    };
    match mode {
        2 => {
            let word_layout = total_bytes == word_bytes;
            let padded_layout =
                total_bytes.is_multiple_of(height) && total_bytes / height >= packed_row;
            match (word_layout, padded_layout) {
                (true, false) => words(),
                (false, true) => packed(false),
                _ => Err(unsupported(format!(
                    "Nikon mode 2 layout is ambiguous or invalid for {total_bytes} bytes"
                ))),
            }
        }
        5 if bits == 12 && width.is_multiple_of(8) => Ok(LayoutKind::StripedPacked12),
        5 => Err(unsupported(
            "Nikon mode 5 requires 12-bit rows with width divisible by eight",
        )),
        6 if bits == 12 => packed(true),
        7 if bits == 12 => words(),
        9 if bits == 12 => packed(false),
        1 | 4 => Err(unsupported(format!(
            "Nikon maker mode {mode} is lossy, deferred by policy"
        ))),
        8 => Err(unsupported("Nikon small raw mode 8 is unsupported")),
        10 if bits == 14 && total_bytes == word_bytes => words(),
        10 if bits == 14 && byte_order == CodecByteOrder::Little => packed(false),
        10 => Err(unsupported(
            "Nikon mode 10 requires 14-bit little-endian packed data or 16-bit words",
        )),
        13 | 14 => Err(unsupported("Nikon HE/HE* modes 13 and 14 are unsupported")),
        _ => Err(unsupported(format!(
            "Nikon maker mode {mode} is unknown or unsupported at {bits} bits"
        ))),
    }
}

fn validate_strip_sizes(
    chunks: &[Chunk],
    kind: &LayoutKind,
    width: usize,
    height: usize,
    rows_per_strip: usize,
) -> Result<(), DecodeError> {
    for (index, chunk) in chunks.iter().enumerate() {
        let start = index
            .checked_mul(rows_per_strip)
            .ok_or_else(|| invalid("Nikon strip row offset overflow"))?;
        let rows = rows_per_strip.min(height - start);
        let required = match kind {
            LayoutKind::Packed { row_stride, .. } => rows
                .checked_mul(*row_stride)
                .ok_or_else(|| invalid("Nikon packed strip size overflow"))?,
            LayoutKind::UnpackedWords { .. } => rows
                .checked_mul(width)
                .and_then(|samples| samples.checked_mul(2))
                .ok_or_else(|| invalid("Nikon word strip size overflow"))?,
            LayoutKind::StripedPacked12 => rows
                .checked_mul(width * 3 / 2)
                .ok_or_else(|| invalid("Nikon mode-5 strip size overflow"))?,
            LayoutKind::NikonHuffman { .. } => {
                if chunk.byte_count == 0 {
                    return Err(invalid("Nikon mode-3 strip must not be empty"));
                }
                continue;
            }
        };
        if chunk.byte_count != required {
            return Err(unsupported(format!(
                "Nikon strip {index} has {} bytes; layout requires {required}",
                chunk.byte_count
            )));
        }
    }
    Ok(())
}

const fn codec_order(order: ByteOrder) -> CodecByteOrder {
    match order {
        ByteOrder::Little => CodecByteOrder::Little,
        ByteOrder::Big => CodecByteOrder::Big,
    }
}

fn required_usize<T>(directory: &Directory, tag: T) -> Result<usize, DecodeError>
where
    T: Into<Tag>,
{
    let tag = tag.into();
    let value = unsigned_scalar(directory.get(&tag))
        .ok_or_else(|| invalid(format!("{tag} is missing or invalid")))?;
    usize::try_from(value).map_err(|_| invalid(format!("{tag} exceeds the address space")))
}

fn optional_usize<T>(directory: &Directory, tag: T) -> Option<usize>
where
    T: Into<Tag>,
{
    usize::try_from(unsigned_scalar(directory.get(&tag.into()))?).ok()
}

fn required_u16<T>(directory: &Directory, tag: T) -> Result<u16, DecodeError>
where
    T: Into<Tag>,
{
    let tag = tag.into();
    u16::try_from(
        unsigned_scalar(directory.get(&tag))
            .ok_or_else(|| invalid(format!("{tag} is missing or invalid")))?,
    )
    .map_err(|_| invalid(format!("{tag} exceeds u16")))
}

fn optional_u16<T>(directory: &Directory, tag: T) -> Option<u16>
where
    T: Into<Tag>,
{
    u16::try_from(unsigned_scalar(directory.get(&tag.into()))?).ok()
}

fn unsigned_vec<T>(directory: &Directory, tag: T) -> Result<Vec<usize>, DecodeError>
where
    T: Into<Tag>,
{
    let tag = tag.into();
    tiff_unsigned_vec(directory, tag)
        .map_err(|error| invalid(format!("{tag} is missing or invalid: {error}")))
}

fn invalid(detail: impl Into<Box<str>>) -> DecodeError { DecodeError::InvalidInput(detail.into()) }

fn unsupported(detail: impl Into<Box<str>>) -> DecodeError {
    DecodeError::Unsupported(detail.into())
}
