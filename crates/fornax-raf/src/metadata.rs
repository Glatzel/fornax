use fornax_dng::metadata::DngTag;
use fornax_raw_core::error::DecodeError;
use fornax_raw_core::metadata::{MetadataSource, ProcessMetadata};
use fornax_tiff::constants::{PhotometricInterpretation, TiffTag};
use fornax_tiff::{Directory, Tag, Value, unsigned_vec};

use crate::container::{Container, RawLayout};

const FUJI_BLACK_LEVEL: Tag = Tag(0xf00a);
const FUJI_WHITE_BALANCE: Tag = Tag(0xf00e);

pub fn map(container: &Container) -> Result<Directory, DecodeError> {
    let layout = &container.layout;
    let (pattern, repeat) = match container.cfa {
        crate::Cfa::Bayer(pattern) => (pattern.to_vec(), [2_u32, 2]),
        crate::Cfa::XTrans(pattern) => (pattern.to_vec(), [6_u32, 6]),
    };
    let width = u32::try_from(layout.width).map_err(|_| invalid("RAF width exceeds u32"))?;
    let height = u32::try_from(layout.height).map_err(|_| invalid("RAF height exceeds u32"))?;
    let active = active_area(layout, container.profile.map(|profile| profile.borders))?;
    let crop = default_crop(container, active)?;
    let (black, black_repeat, black_source) = black_levels(container, layout, repeat)?;
    let white = (1_u32 << layout.bits) - 1;

    let mut directory = Directory::default();
    directory.insert(Tag::from(DngTag::DngVersion), bytes(&[1, 4, 0, 0]));
    directory.insert(Tag::from(DngTag::DngBackwardVersion), bytes(&[1, 4, 0, 0]));
    directory.insert(Tag::from(TiffTag::Make), Value::Ascii("FUJIFILM".into()));
    directory.insert(
        Tag::from(TiffTag::Model),
        Value::Ascii(container.model.clone()),
    );
    directory.insert(
        Tag::from(DngTag::UniqueCameraModel),
        Value::Ascii(format!("Fujifilm {}", container.model)),
    );
    directory.insert(Tag::from(TiffTag::ImageWidth), Value::Unsigned(width));
    directory.insert(Tag::from(TiffTag::ImageLength), Value::Unsigned(height));
    directory.insert(Tag::from(TiffTag::BitsPerSample), Value::Short(16));
    directory.insert(Tag::from(TiffTag::SamplesPerPixel), Value::Short(1));
    directory.insert(Tag::from(TiffTag::SampleFormat), Value::Short(1));
    directory.insert(
        Tag::from(TiffTag::PhotometricInterpretation),
        Value::Short(u16::from(PhotometricInterpretation::Cfa)),
    );
    directory.insert(
        Tag::from(TiffTag::CfaRepeatPatternDim),
        Value::List(repeat.into_iter().map(Value::Unsigned).collect()),
    );
    directory.insert(
        Tag::from(TiffTag::CfaPattern),
        Value::List(pattern.into_iter().map(Value::Byte).collect()),
    );
    directory.insert(
        Tag::from(DngTag::CfaPlaneColor),
        Value::List(vec![Value::Byte(0), Value::Byte(1), Value::Byte(2)]),
    );
    directory.insert(Tag::from(DngTag::CfaLayout), Value::Short(1));
    directory.insert(
        Tag::from(DngTag::ActiveArea),
        Value::List(vec![
            Value::Unsigned(active.y),
            Value::Unsigned(active.x),
            Value::Unsigned(active.y + active.height),
            Value::Unsigned(active.x + active.width),
        ]),
    );
    directory.insert(
        Tag::from(DngTag::DefaultCropOrigin),
        Value::List(vec![Value::Unsigned(crop.x), Value::Unsigned(crop.y)]),
    );
    directory.insert(
        Tag::from(DngTag::DefaultCropSize),
        Value::List(vec![
            Value::Unsigned(crop.width),
            Value::Unsigned(crop.height),
        ]),
    );
    directory.insert(
        Tag::from(DngTag::BlackLevelRepeatDim),
        Value::List(black_repeat.into_iter().map(Value::Short).collect()),
    );
    directory.insert(
        Tag::from(DngTag::BlackLevel),
        Value::List(black.into_iter().map(Value::Short).collect()),
    );
    directory.insert(Tag::from(DngTag::WhiteLevel), Value::Unsigned(white));
    if let Some(neutral) = neutral(container)? {
        directory.insert(
            Tag::from(DngTag::AsShotNeutral),
            Value::List(neutral.into_iter().map(rational).collect()),
        );
    }
    let _ = black_source;
    Ok(directory)
}

pub fn process(
    metadata: &Directory,
    width: u32,
    height: u32,
) -> Result<ProcessMetadata, DecodeError> {
    fornax_dng::dng_decoder::process_metadata_from_dng_metadata(width, height, metadata)
}

fn active_area(
    layout: &RawLayout,
    borders: Option<[u32; 4]>,
) -> Result<image::math::Rect, DecodeError> {
    let [left, top, right, bottom] = borders.unwrap_or([0, 0, 0, 0]);
    let width = u32::try_from(layout.width).map_err(|_| invalid("RAF width exceeds u32"))?;
    let height = u32::try_from(layout.height).map_err(|_| invalid("RAF height exceeds u32"))?;
    let active_width = width
        .checked_sub(left)
        .and_then(|value| value.checked_sub(right))
        .ok_or_else(|| invalid("RAF active-area horizontal borders are invalid"))?;
    let active_height = height
        .checked_sub(top)
        .and_then(|value| value.checked_sub(bottom))
        .ok_or_else(|| invalid("RAF active-area vertical borders are invalid"))?;
    if active_width == 0 || active_height == 0 {
        return Err(invalid("RAF active area must not be empty"));
    }
    Ok(image::math::Rect {
        x: left,
        y: top,
        width: active_width,
        height: active_height,
    })
}

fn default_crop(
    container: &Container,
    active: image::math::Rect,
) -> Result<image::math::Rect, DecodeError> {
    let (Some([top, left]), Some([height, width])) =
        (container.tags.crop_origin, container.tags.cropped_size)
    else {
        return Ok(image::math::Rect {
            x: 0,
            y: 0,
            width: active.width,
            height: active.height,
        });
    };
    let x = u32::from(left)
        .checked_sub(active.x)
        .ok_or_else(|| invalid("RAF crop origin is left of ActiveArea"))?;
    let y = u32::from(top)
        .checked_sub(active.y)
        .ok_or_else(|| invalid("RAF crop origin is above ActiveArea"))?;
    let width = u32::from(width);
    let height = u32::from(height);
    if x.checked_add(width)
        .is_none_or(|right| right > active.width)
        || y.checked_add(height)
            .is_none_or(|bottom| bottom > active.height)
    {
        return Err(invalid("RAF crop is outside ActiveArea"));
    }
    Ok(image::math::Rect {
        x,
        y,
        width,
        height,
    })
}

fn black_levels(
    container: &Container,
    layout: &RawLayout,
    cfa_repeat: [u32; 2],
) -> Result<(Vec<u16>, [u16; 2], MetadataSource), DecodeError> {
    let Some(value) = container.fuji_ifd.get(&FUJI_BLACK_LEVEL) else {
        return Ok((vec![0], [1, 1], MetadataSource::Default));
    };
    let values = unsigned_vec::<_, u64>(&container.fuji_ifd, FUJI_BLACK_LEVEL)
        .map_err(|_| invalid("RAF BlackLevel is invalid"))?
        .into_iter()
        .map(u16::try_from)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| invalid("RAF BlackLevel exceeds u16"))?;
    let repeat = if values.len() == 1 {
        [1, 1]
    } else {
        let entries = (|| {
            let width = usize::try_from(cfa_repeat[0]).ok()?;
            let height = usize::try_from(cfa_repeat[1]).ok()?;
            width.checked_mul(height)
        })()
        .ok_or_else(|| invalid("RAF CFA repeat dimensions overflow"))?;
        if values.len() != entries {
            return Err(invalid(format!(
                "RAF has {} black levels for {}x{} CFA",
                values.len(),
                cfa_repeat[0],
                cfa_repeat[1]
            )));
        }
        [
            u16::try_from(cfa_repeat[0]).map_err(|_| invalid("RAF CFA width exceeds u16"))?,
            u16::try_from(cfa_repeat[1]).map_err(|_| invalid("RAF CFA height exceeds u16"))?,
        ]
    };
    let _ = value;
    let _ = layout;
    Ok((
        values,
        repeat,
        MetadataSource::Tag {
            tag: FUJI_BLACK_LEVEL.0,
            version: None,
        },
    ))
}

fn neutral(container: &Container) -> Result<Option<[f64; 3]>, DecodeError> {
    let Some(_) = container.fuji_ifd.get(&FUJI_WHITE_BALANCE) else {
        return Ok(None);
    };
    let values = unsigned_vec::<_, u64>(&container.fuji_ifd, FUJI_WHITE_BALANCE)
        .map_err(|_| invalid("RAF white-balance tag is invalid"))?;
    let [red, green, blue, ..] = values.as_slice() else {
        return Ok(None);
    };
    if *green == 0 {
        return Err(invalid("RAF green white-balance level is zero"));
    }
    Ok(Some([
        *red as f64 / *green as f64,
        1.0,
        *blue as f64 / *green as f64,
    ]))
}

fn bytes(values: &[u8]) -> Value { Value::List(values.iter().copied().map(Value::Byte).collect()) }

fn rational(value: f64) -> Value {
    const DENOMINATOR: u32 = 1_000_000;
    Value::Rational((value * f64::from(DENOMINATOR)).round() as u32, DENOMINATOR)
}

fn invalid(detail: impl Into<Box<str>>) -> DecodeError { DecodeError::InvalidInput(detail.into()) }
