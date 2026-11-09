use fornax_core::error::DecodeError;
use fornax_dng::metadata::DngTag;
use fornax_tiff::constants::{PhotometricInterpretation, TiffTag};
use fornax_tiff::{ByteOrder, Directory, ParsedTiff, Tag, Value, unsigned_scalar};

use crate::container::{
    CompressionFamily, RawLayout, SONY_BLACK_LEVEL_1, SONY_CURVE, SONY_GRBG_WHITE_BALANCE,
    SONY_KEY, SONY_LENGTH, SONY_OFFSET, SONY_RAW_IMAGE_SIZE, SONY_RGGB_BLACK_LEVEL,
    SONY_RGGB_WHITE_BALANCE, SONY_WHITE_LEVEL, bytes, invalid, optional_usize, unsigned_vec,
};
use crate::sony;

pub fn map(
    parsed: &ParsedTiff,
    raw_ifd: usize,
    layout: &RawLayout,
) -> Result<Directory, DecodeError> {
    let raw = parsed
        .directories
        .get(raw_ifd)
        .ok_or_else(|| invalid("selected Sony raw IFD is missing"))?;
    let private = private_ifd(parsed)?;
    let mut metadata = parsed.root.clone();
    metadata.extend(raw.clone());
    metadata.remove(&Tag::from(DngTag::DngPrivateData));

    let cfa = cfa_pattern(raw);
    let active = active_area(raw, layout)?;
    let crop = default_crop(raw, active, &model(&metadata))?;
    let black = black_levels(private.as_ref(), layout);
    let white = white_level(private.as_ref(), raw, layout)?;
    let neutral = neutral(private.as_ref());

    metadata.insert(
        Tag::from(DngTag::DngVersion),
        Value::List(vec![
            Value::Byte(1),
            Value::Byte(4),
            Value::Byte(0),
            Value::Byte(0),
        ]),
    );
    metadata.insert(
        Tag::from(DngTag::DngBackwardVersion),
        Value::List(vec![
            Value::Byte(1),
            Value::Byte(4),
            Value::Byte(0),
            Value::Byte(0),
        ]),
    );
    metadata.insert(
        Tag::from(TiffTag::ImageWidth),
        Value::Unsigned(u32::try_from(layout.width).map_err(|_| invalid("ARW width exceeds u32"))?),
    );
    metadata.insert(
        Tag::from(TiffTag::ImageLength),
        Value::Unsigned(
            u32::try_from(layout.stored_height).map_err(|_| invalid("ARW height exceeds u32"))?,
        ),
    );
    metadata.insert(Tag::from(TiffTag::BitsPerSample), Value::Short(16));
    metadata.insert(Tag::from(TiffTag::SamplesPerPixel), Value::Short(1));
    metadata.insert(Tag::from(TiffTag::SampleFormat), Value::Short(1));
    metadata.insert(
        Tag::from(TiffTag::PhotometricInterpretation),
        Value::Short(u16::from(PhotometricInterpretation::Cfa)),
    );
    metadata.insert(
        Tag::from(TiffTag::CfaRepeatPatternDim),
        Value::List(vec![Value::Short(2), Value::Short(2)]),
    );
    metadata.insert(
        Tag::from(TiffTag::CfaPattern),
        Value::List(cfa.iter().copied().map(Value::Byte).collect()),
    );
    metadata.insert(
        Tag::from(DngTag::CfaPlaneColor),
        Value::List(vec![Value::Byte(0), Value::Byte(1), Value::Byte(2)]),
    );
    metadata.insert(Tag::from(DngTag::CfaLayout), Value::Short(1));
    metadata.insert(
        Tag::from(DngTag::ActiveArea),
        Value::List(
            [active.0, active.1, active.2, active.3]
                .into_iter()
                .map(|value| {
                    u32::try_from(value)
                        .map(Value::Unsigned)
                        .map_err(|_| invalid("ARW active area exceeds u32"))
                })
                .collect::<Result<Vec<_>, _>>()?,
        ),
    );
    metadata.insert(
        Tag::from(DngTag::DefaultCropOrigin),
        Value::List(vec![Value::Unsigned(crop.0), Value::Unsigned(crop.1)]),
    );
    metadata.insert(
        Tag::from(DngTag::DefaultCropSize),
        Value::List(vec![Value::Unsigned(crop.2), Value::Unsigned(crop.3)]),
    );
    metadata.insert(
        Tag::from(DngTag::BlackLevelRepeatDim),
        Value::List(vec![Value::Short(2), Value::Short(2)]),
    );
    metadata.insert(
        Tag::from(DngTag::BlackLevel),
        Value::List(black.into_iter().map(Value::Short).collect()),
    );
    metadata.insert(
        Tag::from(DngTag::WhiteLevel),
        Value::Unsigned(u32::from(white)),
    );
    if let Some(neutral) = neutral {
        metadata.insert(
            Tag::from(DngTag::AsShotNeutral),
            Value::List(neutral.into_iter().map(rational).collect()),
        );
    }
    if !metadata.contains_key(&Tag::from(DngTag::UniqueCameraModel))
        && let Some(Value::Ascii(model)) = metadata.get(&Tag::from(TiffTag::Model))
    {
        metadata.insert(
            Tag::from(DngTag::UniqueCameraModel),
            Value::Ascii(format!("Sony {model}")),
        );
    }
    Ok(metadata)
}

fn private_ifd(parsed: &ParsedTiff) -> Result<Option<Directory>, DecodeError> {
    let Some(value) = parsed.root.get(&Tag::from(DngTag::DngPrivateData)) else {
        return Ok(None);
    };
    let raw = bytes(value).ok_or_else(|| invalid("DNGPrivateData is not a byte string"))?;
    let offset = raw
        .get(..4)
        .and_then(|bytes| bytes.try_into().ok())
        .map(u32::from_le_bytes)
        .ok_or_else(|| invalid("DNGPrivateData does not contain a four-byte IFD offset"))?;
    let outer = fornax_tiff::parse_ifd_at(&parsed.source, u64::from(offset), ByteOrder::Little, 0)
        .map_err(|error| invalid(format!("parsing Sony private IFD failed: {error}")))?;
    let Some(offset) = optional_usize(&outer, SONY_OFFSET) else {
        return Ok(None);
    };
    let Some(length) = optional_usize(&outer, SONY_LENGTH) else {
        return Ok(None);
    };
    let key = outer
        .get(&SONY_KEY)
        .and_then(bytes)
        .and_then(|bytes| {
            bytes.get(..4).map(|value| {
                u32::from_le_bytes(value.try_into().expect("the byte slice is four bytes"))
            })
        })
        .or_else(|| {
            let value = unsigned_scalar(outer.get(&SONY_KEY))?;
            u32::try_from(value).ok()
        })
        .ok_or_else(|| invalid("Sony private IFD has no decryption key"))?;
    let decrypted = sony::decrypt(&parsed.source, offset, length, key)?;
    let correction =
        -i64::try_from(offset).map_err(|_| invalid("Sony private payload offset exceeds i64"))?;
    let inner = fornax_tiff::parse_ifd_at(&decrypted, 0, ByteOrder::Little, correction)
        .map_err(|error| invalid(format!("parsing decrypted Sony metadata failed: {error}")))?;
    Ok(Some(inner))
}

fn cfa_pattern(raw: &Directory) -> [u8; 4] {
    unsigned_vec(raw, TiffTag::CfaPattern)
        .ok()
        .and_then(|values| values.try_into().ok())
        .and_then(|values: [usize; 4]| {
            let values = values.map(u8::try_from);
            let [a, b, c, d] = values;
            Some([a.ok()?, b.ok()?, c.ok()?, d.ok()?])
        })
        .filter(|pattern| {
            pattern.iter().filter(|value| **value == 1).count() == 2
                && pattern.contains(&0)
                && pattern.contains(&2)
        })
        .unwrap_or([0, 1, 1, 2])
}

fn active_area(
    raw: &Directory,
    layout: &RawLayout,
) -> Result<(usize, usize, usize, usize), DecodeError> {
    let values = unsigned_vec(raw, SONY_RAW_IMAGE_SIZE).ok();
    let (top, left, bottom, right) = match values.as_deref() {
        Some(&[width, height] | &[width, height, ..]) if width > 0 && height > 0 => {
            (0, 0, height, width)
        }
        Some(&[top, left, bottom, right]) => (top, left, bottom, right),
        _ => (0, 0, layout.stored_height, layout.width),
    };
    if top >= bottom || left >= right || bottom > layout.stored_height || right > layout.width {
        return Err(invalid(format!(
            "Sony active area ({top}, {left}, {bottom}, {right}) is outside {}x{}",
            layout.width, layout.stored_height
        )));
    }
    Ok((top, left, bottom, right))
}

fn default_crop(
    raw: &Directory,
    active: (usize, usize, usize, usize),
    model: &str,
) -> Result<(u32, u32, u32, u32), DecodeError> {
    let origin = unsigned_vec(raw, DngTag::DefaultCropOrigin).ok();
    let size = unsigned_vec(raw, DngTag::DefaultCropSize).ok();
    if let (Some(origin), Some(size)) = (origin, size) {
        let [x, y]: [usize; 2] = origin
            .try_into()
            .map_err(|_| invalid("DefaultCropOrigin must contain two values"))?;
        let [width, height]: [usize; 2] = size
            .try_into()
            .map_err(|_| invalid("DefaultCropSize must contain two values"))?;
        return crop_values(active, x, y, width, height);
    }

    let borders = match model {
        "DSLR-A700" => Some([4, 4, 12, 4]),
        "NEX-5" => Some([8, 8, 8, 8]),
        "ILCE-7M3" => Some([12, 12, 36, 12]),
        _ => None,
    };
    if let Some([left, top, right, bottom]) = borders {
        return crop_values(
            active,
            left,
            top,
            active.2 - active.0 - top - bottom,
            active.3 - active.1 - left - right,
        );
    }
    crop_values(active, 0, 0, active.3 - active.1, active.2 - active.0)
}

fn crop_values(
    active: (usize, usize, usize, usize),
    x: usize,
    y: usize,
    mut width: usize,
    mut height: usize,
) -> Result<(u32, u32, u32, u32), DecodeError> {
    let active_width = active.3 - active.1;
    let active_height = active.2 - active.0;
    let original_fits =
        x.saturating_add(width) <= active_width && y.saturating_add(height) <= active_height;
    if !original_fits {
        // Sony stores some portrait crops in oriented coordinates while the raw
        // IFD remains landscape.
        (width, height) = (height, width);
    }
    if width == 0
        || height == 0
        || x.saturating_add(width) > active_width
        || y.saturating_add(height) > active_height
    {
        return Err(invalid(format!(
            "DefaultCrop {width}x{height} at ({x}, {y}) is outside the {active_width}x{active_height} active area"
        )));
    }
    Ok((
        u32::try_from(x).map_err(|_| invalid("DefaultCropOrigin exceeds u32"))?,
        u32::try_from(y).map_err(|_| invalid("DefaultCropOrigin exceeds u32"))?,
        u32::try_from(width).map_err(|_| invalid("DefaultCropSize exceeds u32"))?,
        u32::try_from(height).map_err(|_| invalid("DefaultCropSize exceeds u32"))?,
    ))
}

fn black_levels(private: Option<&Directory>, layout: &RawLayout) -> [u16; 4] {
    let Some(private) = private else {
        return [0; 4];
    };
    let values = [SONY_RGGB_BLACK_LEVEL, SONY_BLACK_LEVEL_1]
        .into_iter()
        .find_map(|tag| unsigned_vec(private, tag).ok())
        .unwrap_or_default();
    let mut levels = [0_u16; 4];
    match values.as_slice() {
        [] => [0; 4],
        [value] => {
            levels.fill(u16::try_from(*value).unwrap_or(0));
            levels
        }
        _ => {
            for (level, value) in levels.iter_mut().zip(values) {
                *level = u16::try_from(value).unwrap_or(0);
            }
            if layout.compression == CompressionFamily::Uncompressed && layout.bits_per_sample == 12
            {
                // Older ARWs declare 14-bit levels but store 12-bit samples.
                for value in &mut levels {
                    *value >>= 2;
                }
            }
            levels
        }
    }
}

fn white_level(
    private: Option<&Directory>,
    raw: &Directory,
    layout: &RawLayout,
) -> Result<u16, DecodeError> {
    if let Some(value) = private
        .and_then(|directory| directory.get(&SONY_WHITE_LEVEL))
        .and_then(|value| value.clone().into_u64_vec().ok())
        .and_then(|values| values.first().copied())
    {
        return u16::try_from(value).map_err(|_| invalid("Sony white level exceeds u16"));
    }
    Ok(match layout.compression {
        CompressionFamily::Arw2 => {
            let curve = curve_knots(raw).map_or_else(
                || (0..4096).collect(),
                |knots| flashraw_codecs::sony_arw2::build_curve(&knots),
            );
            curve.last().copied().unwrap_or(16_383)
        }
        CompressionFamily::Arw6 => flashraw_codecs::sony_arw6_curve::LLVC3_CURVE[4095],
        CompressionFamily::Arw1 => 4095,
        CompressionFamily::Uncompressed => match layout.bits_per_sample {
            12 => 4095,
            16 => u16::MAX,
            bits => ((1_u32 << bits) - 1).min(u32::from(u16::MAX)) as u16,
        },
        CompressionFamily::SonyLosslessJpeg => {
            ((1_u32 << layout.bits_per_sample) - 1).min(u32::from(u16::MAX)) as u16
        }
    })
}

pub fn curve_knots(raw: &Directory) -> Option<[u16; 4]> {
    let values = unsigned_vec(raw, SONY_CURVE).ok()?;
    let values: [u16; 4] = values
        .iter()
        .take(4)
        .map(|value| u16::try_from(*value).ok())
        .collect::<Option<Vec<_>>>()?
        .try_into()
        .ok()?;
    Some(values)
}

fn neutral(private: Option<&Directory>) -> Option<[f64; 3]> {
    let private = private?;
    let mut gains = [0.0_f64; 4];
    if let Ok(values) = unsigned_vec(private, SONY_GRBG_WHITE_BALANCE) {
        if values.len() >= 4 {
            gains = [
                values[1] as f64,
                values[0] as f64,
                values[3] as f64,
                values[2] as f64,
            ];
        }
    } else if let Ok(values) = unsigned_vec(private, SONY_RGGB_WHITE_BALANCE)
        && values.len() >= 4
    {
        gains = values[..4]
            .iter()
            .map(|value| *value as f64)
            .collect::<Vec<_>>()
            .try_into()
            .expect("the slice is four values");
    }
    if !gains.iter().all(|gain| gain.is_finite() && *gain > 0.0) {
        return None;
    }
    let green = f64::midpoint(gains[1], gains[2]);
    Some([green / gains[0], 1.0, green / gains[3]])
}

fn rational(value: f64) -> Value {
    const DENOMINATOR: u32 = 1_000_000;
    Value::Rational((value * f64::from(DENOMINATOR)).round() as u32, DENOMINATOR)
}

fn model(metadata: &Directory) -> String {
    metadata
        .get(&Tag::from(TiffTag::Model))
        .and_then(|value| match value {
            Value::Ascii(value) => Some(value.trim().to_owned()),
            _ => None,
        })
        .unwrap_or_default()
}
