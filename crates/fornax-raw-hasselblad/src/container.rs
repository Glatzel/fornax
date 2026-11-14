use std::sync::Arc;

use fornax_dng::metadata::DngTag;
use fornax_raw_core::error::DecodeError;
use fornax_tiff::constants::{PhotometricInterpretation, TiffTag};
use fornax_tiff::{ByteOrder, Directory, ParsedTiff, Tag, Value, unsigned_scalar, unsigned_vec};
use image::math::Rect;

use crate::profiles::{ModelProfile, profile};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Compression {
    Uncompressed,
    LosslessJpeg,
}

#[derive(Clone, Debug)]
pub struct RawLayout {
    pub compression: Compression,
    pub width: usize,
    pub height: usize,
    pub offset: usize,
    pub byte_count: usize,
}

#[derive(Clone, Debug)]
pub struct Cfa {
    pub repeat: [u32; 2],
    pub pattern: Vec<u8>,
    pub plane_colors: Vec<u8>,
    pub layout: u16,
}

#[derive(Clone, Debug)]
pub struct Container {
    pub source: Arc<[u8]>,
    pub make: String,
    pub model: String,
    pub unique_model: String,
    pub cfa: Cfa,
    pub active_area: Rect,
    pub root: Directory,
    pub raw: Directory,
    pub exif: Directory,
    pub layout: RawLayout,
}

pub fn parse_container(source: &[u8]) -> Result<Container, DecodeError> {
    if !source.starts_with(b"II") {
        return Err(unsupported(
            "3FR requires little-endian classic TIFF sample words",
        ));
    }
    let parsed = fornax_tiff::parse(source)
        .map_err(|error| invalid(format!("parsing 3FR TIFF failed: {error}")))?;
    let make = ascii(&parsed.root(), TiffTag::Make).unwrap_or_default();
    if !make.eq_ignore_ascii_case("Hasselblad") {
        return Err(unsupported(format!(
            "3FR container is not a Hasselblad image (Make={make:?})"
        )));
    }
    let model = ascii(&parsed.root(), TiffTag::Model)
        .ok_or_else(|| invalid("3FR root IFD has no ASCII Model"))?;
    let unique_model =
        ascii_tag(&parsed.root(), DngTag::UniqueCameraModel).unwrap_or_else(|| model.clone());
    let profile = profile(&unique_model, &model);
    let raw_index = select_raw_ifd(&parsed)?;
    let raw = parsed
        .directories
        .get(raw_index)
        .ok_or_else(|| invalid("selected 3FR raw IFD is missing"))?;
    let cfa = source_cfa(raw)?.or_else(|| {
        profile.map(|profile| Cfa {
            repeat: [2, 2],
            pattern: profile.cfa.to_vec(),
            plane_colors: vec![0, 1, 2],
            layout: 1,
        })
    });
    let cfa = cfa.ok_or_else(|| {
        unsupported(format!(
            "3FR camera model {unique_model:?} has no conventional CFA profile"
        ))
    })?;
    let active_area = active_area(raw, profile)?;
    let layout = raw_layout(source.len(), raw)?;
    let exif = exif_directory(source, &parsed.root())?;
    Ok(Container {
        source: Arc::from(source),
        make,
        model,
        unique_model,
        cfa,
        active_area,
        root: parsed.root().clone(),
        raw: raw.clone(),
        exif,
        layout,
    })
}

fn select_raw_ifd(parsed: &ParsedTiff) -> Result<usize, DecodeError> {
    parsed
        .directories
        .iter()
        .enumerate()
        .filter(|(_, directory)| {
            let reduced = unsigned_scalar(directory.get(&Tag::from(TiffTag::NewSubfileType)))
                .is_some_and(|value| value & 1 != 0);
            let photometric =
                unsigned_scalar(directory.get(&Tag::from(TiffTag::PhotometricInterpretation)))
                    .and_then(|value| u16::try_from(value).ok())
                    .is_some_and(|value| value == u16::from(PhotometricInterpretation::Cfa));
            !reduced
                && photometric
                && directory.contains_key(&Tag::from(DngTag::WhiteLevel))
                && directory.contains_key(&Tag::from(TiffTag::StripOffsets))
                && directory.contains_key(&Tag::from(TiffTag::StripByteCounts))
        })
        .max_by_key(|(_, directory)| {
            optional_usize(directory, TiffTag::ImageWidth)
                .unwrap_or(0)
                .saturating_mul(optional_usize(directory, TiffTag::ImageLength).unwrap_or(0))
        })
        .map(|(index, _)| index)
        .ok_or_else(|| unsupported("3FR has no full-resolution CFA SubIFD with WhiteLevel"))
}

fn source_cfa(raw: &Directory) -> Result<Option<Cfa>, DecodeError> {
    let repeat = raw.get(&Tag::from(TiffTag::CfaRepeatPatternDim));
    let pattern = raw.get(&Tag::from(TiffTag::CfaPattern));
    match (repeat, pattern) {
        (None, None) => Ok(None),
        (Some(_), None) | (None, Some(_)) => Err(invalid(
            "3FR CFA repeat and pattern tags must appear together",
        )),
        (Some(_), Some(_)) => {
            let repeat: [u32; 2] = unsigned_values(raw, TiffTag::CfaRepeatPatternDim)?
                .try_into()
                .map_err(|_| invalid("3FR CFARepeatPatternDim must contain two values"))?;
            let pattern = unsigned_values(raw, TiffTag::CfaPattern)?
                .into_iter()
                .map(|value| u8::try_from(value).map_err(|_| invalid("3FR CFA plane exceeds u8")))
                .collect::<Result<Vec<_>, _>>()?;
            let width =
                usize::try_from(repeat[0]).map_err(|_| invalid("3FR CFA width exceeds usize"))?;
            let height =
                usize::try_from(repeat[1]).map_err(|_| invalid("3FR CFA height exceeds usize"))?;
            let expected = width
                .checked_mul(height)
                .ok_or_else(|| invalid("3FR CFA dimensions overflow"))?;
            if expected == 0 || pattern.len() != expected {
                return Err(invalid(format!(
                    "3FR CFA pattern has {} values for {expected} cells",
                    pattern.len()
                )));
            }
            let plane_colors = raw
                .get(&Tag::from(DngTag::CfaPlaneColor))
                .map(|_| unsigned_values(raw, DngTag::CfaPlaneColor))
                .transpose()?
                .unwrap_or_else(|| vec![0, 1, 2])
                .into_iter()
                .map(|value| {
                    u8::try_from(value).map_err(|_| invalid("3FR CFA plane color exceeds u8"))
                })
                .collect::<Result<Vec<_>, _>>()?;
            if plane_colors.is_empty() || pattern.iter().any(|plane| !plane_colors.contains(plane))
            {
                return Err(invalid("3FR CFA pattern uses an undefined plane color"));
            }
            let layout = optional_usize(raw, DngTag::CfaLayout)
                .unwrap_or(1)
                .try_into()
                .map_err(|_| invalid("3FR CFALayout exceeds u16"))?;
            Ok(Some(Cfa {
                repeat,
                pattern,
                plane_colors,
                layout,
            }))
        }
    }
}

fn active_area(raw: &Directory, profile: Option<ModelProfile>) -> Result<Rect, DecodeError> {
    if raw.contains_key(&Tag::from(DngTag::ActiveArea)) {
        let [top, left, bottom, right]: [u32; 4] = unsigned_values(raw, DngTag::ActiveArea)?
            .try_into()
            .map_err(|_| invalid("3FR ActiveArea must contain four values"))?;
        return checked_rect(left, top, right, bottom);
    }
    let width = required_usize(raw, TiffTag::ImageWidth)?;
    let height = required_usize(raw, TiffTag::ImageLength)?;
    let [left, top, right, bottom] = profile.map_or([0; 4], |profile| profile.borders);
    let right = width
        .checked_sub(usize::try_from(right).map_err(|_| invalid("3FR right border exceeds usize"))?)
        .ok_or_else(|| invalid("3FR right active border exceeds image width"))?;
    let bottom = height
        .checked_sub(
            usize::try_from(bottom).map_err(|_| invalid("3FR bottom border exceeds usize"))?,
        )
        .ok_or_else(|| invalid("3FR bottom active border exceeds image height"))?;
    checked_rect(
        left,
        top,
        u32::try_from(right).map_err(|_| invalid("3FR active width exceeds u32"))?,
        u32::try_from(bottom).map_err(|_| invalid("3FR active height exceeds u32"))?,
    )
}

fn checked_rect(left: u32, top: u32, right: u32, bottom: u32) -> Result<Rect, DecodeError> {
    if left >= right || top >= bottom {
        return Err(invalid("3FR ActiveArea must not be empty"));
    }
    Ok(Rect {
        x: left,
        y: top,
        width: right - left,
        height: bottom - top,
    })
}

fn raw_layout(source_len: usize, raw: &Directory) -> Result<RawLayout, DecodeError> {
    let width = required_usize(raw, TiffTag::ImageWidth)?;
    let height = required_usize(raw, TiffTag::ImageLength)?;
    if width == 0 || height == 0 {
        return Err(invalid("3FR raw dimensions must not be zero"));
    }
    if optional_usize(raw, TiffTag::SamplesPerPixel).unwrap_or(1) != 1 {
        return Err(unsupported("3FR multi-sample raw images are unsupported"));
    }
    if optional_usize(raw, TiffTag::PlanarConfiguration).unwrap_or(1) != 1 {
        return Err(unsupported("3FR planar raw storage is unsupported"));
    }
    if optional_usize(raw, TiffTag::SampleFormat).unwrap_or(1) != 1 {
        return Err(unsupported("3FR non-integer raw samples are unsupported"));
    }
    if required_usize(raw, TiffTag::BitsPerSample)? != 16 {
        return Err(unsupported(
            "3FR sample depths other than 16 bits are unsupported",
        ));
    }
    if raw.contains_key(&Tag::from(TiffTag::TileOffsets)) {
        return Err(unsupported("tiled 3FR raw images are unsupported"));
    }
    let offset = required_usize(raw, TiffTag::StripOffsets)?;
    let declared_count = required_usize(raw, TiffTag::StripByteCounts)?;
    let pixels = width
        .checked_mul(height)
        .ok_or_else(|| invalid("3FR sample count overflow"))?;
    let word_bytes = pixels
        .checked_mul(2)
        .ok_or_else(|| invalid("3FR word byte count overflow"))?;
    let compression = match required_usize(raw, TiffTag::Compression)? {
        1 => {
            if declared_count != word_bytes
                || offset
                    .checked_add(word_bytes)
                    .is_none_or(|end| end > source_len)
            {
                return Err(invalid(
                    "3FR uncompressed payload must contain exactly one 16-bit word per sample",
                ));
            }
            Compression::Uncompressed
        }
        7 => {
            if offset >= source_len {
                return Err(invalid("3FR compressed payload offset is outside the file"));
            }
            Compression::LosslessJpeg
        }
        value => {
            return Err(unsupported(format!(
                "3FR TIFF compression {value} is unsupported"
            )));
        }
    };
    Ok(RawLayout {
        compression,
        width,
        height,
        offset,
        byte_count: source_len - offset,
    })
}

fn exif_directory(source: &[u8], root: &Directory) -> Result<Directory, DecodeError> {
    let Some(pointer) = unsigned_scalar(root.get(&Tag::from(TiffTag::ExifIfdPointer))) else {
        return Ok(Directory::default());
    };
    let byte_order = match &source[..2] {
        b"II" => ByteOrder::Little,
        b"MM" => ByteOrder::Big,
        _ => return Err(invalid("3FR TIFF byte-order marker is invalid")),
    };
    fornax_tiff::parse_ifd_at(source, pointer, byte_order, 0)
        .map_err(|error| invalid(format!("parsing 3FR Exif IFD failed: {error}")))
}

fn ascii(directory: &Directory, tag: TiffTag) -> Option<String> {
    match directory.get(&Tag::from(tag)) {
        Some(Value::Ascii(value)) => Some(value.trim().to_owned()),
        _ => None,
    }
}

fn ascii_tag(directory: &Directory, tag: DngTag) -> Option<String> {
    match directory.get(&Tag::from(tag)) {
        Some(Value::Ascii(value)) => Some(value.trim().to_owned()),
        _ => None,
    }
}

fn required_usize<T>(directory: &Directory, tag: T) -> Result<usize, DecodeError>
where
    T: Into<Tag>,
{
    let tag = tag.into();
    unsigned_scalar(directory.get(&tag))
        .and_then(|value| usize::try_from(value).ok())
        .ok_or_else(|| invalid(format!("3FR {tag} is missing or invalid")))
}

fn optional_usize<T>(directory: &Directory, tag: T) -> Option<usize>
where
    T: Into<Tag>,
{
    usize::try_from(unsigned_scalar(directory.get(&tag.into()))?).ok()
}

fn unsigned_values<T>(directory: &Directory, tag: T) -> Result<Vec<u32>, DecodeError>
where
    T: Into<Tag>,
{
    unsigned_vec::<_, u32>(directory, tag)
        .map_err(|error| invalid(format!("3FR TIFF value is invalid: {error}")))
}

fn invalid(detail: impl Into<Box<str>>) -> DecodeError { DecodeError::InvalidInput(detail.into()) }

fn unsupported(detail: impl Into<Box<str>>) -> DecodeError {
    DecodeError::Unsupported(detail.into())
}
