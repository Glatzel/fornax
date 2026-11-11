use fornax_dng::metadata::DngTag;
use fornax_raw_core::error::DecodeError;
use fornax_raw_core::metadata::ProcessMetadata;
use fornax_tiff::constants::{PhotometricInterpretation, TiffTag};
use fornax_tiff::{Directory, Tag, Value, unsigned_scalar};
use image::math::Rect;

use crate::container::{self, Container};

const ACTIVE_TOP: Tag = Tag(0x0004);
const ACTIVE_LEFT: Tag = Tag(0x0005);
const ACTIVE_BOTTOM: Tag = Tag(0x0006);
const ACTIVE_RIGHT: Tag = Tag(0x0007);
const LINEARITY_RED: Tag = Tag(0x000e);
const LINEARITY_GREEN: Tag = Tag(0x000f);
const LINEARITY_BLUE: Tag = Tag(0x0010);
const WB_RED: Tag = Tag(0x0011);
const WB_BLUE: Tag = Tag(0x0012);
const BLACK_RED: Tag = Tag(0x001c);
const BLACK_GREEN: Tag = Tag(0x001d);
const BLACK_BLUE: Tag = Tag(0x001e);
const WB_RED_LEVEL: Tag = Tag(0x0024);
const WB_GREEN_LEVEL: Tag = Tag(0x0025);
const WB_BLUE_LEVEL: Tag = Tag(0x0026);
const CROP_TOP: Tag = Tag(0x002f);
const CROP_LEFT: Tag = Tag(0x0030);
const CROP_BOTTOM: Tag = Tag(0x0031);
const CROP_RIGHT: Tag = Tag(0x0032);
const EXIF_IFD: Tag = Tag(0x8769);
const CAMERA_SERIAL: Tag = Tag(0xa431);
const LENS_MODEL: Tag = Tag(0xa434);

pub struct Mapped {
    pub metadata: Directory,
    pub black_pattern: [u16; 4],
}

pub fn map(container: &Container) -> Result<Mapped, DecodeError> {
    let raw = &container.parsed.directories[container.raw_ifd];
    let width = u32::try_from(container.layout.width)
        .map_err(|_| container::invalid("RW2 width exceeds u32"))?;
    let height = u32::try_from(container.layout.height)
        .map_err(|_| container::invalid("RW2 height exceeds u32"))?;
    let active = active_area(raw, width, height)?;
    let crop = default_crop(raw, active)?;
    let pattern = shifted_pattern(container.cfa, active.x, active.y);
    let black = black_levels(raw, container)?;
    let black_pattern = spatial_black(black, pattern);
    let white = white_level(raw, container.layout.bits)?;
    let neutral = neutral(raw)?;
    let orientation = orientation(raw)?;
    let exif = exif(raw, container.parsed.source.as_ref())?;
    let serial = ascii(
        exif.as_ref()
            .and_then(|directory| directory.get(&CAMERA_SERIAL)),
    );
    let lens = ascii(
        exif.as_ref()
            .and_then(|directory| directory.get(&LENS_MODEL)),
    );
    let make = ascii(raw.get(&Tag::from(TiffTag::Make))).unwrap_or_else(|| "Panasonic".into());
    let model = ascii(raw.get(&Tag::from(TiffTag::Model)))
        .ok_or_else(|| container::invalid("RW2 camera model is missing"))?;

    let mut metadata = Directory::default();
    metadata.insert(Tag::from(DngTag::DngVersion), bytes(&[1, 4, 0, 0]));
    metadata.insert(Tag::from(DngTag::DngBackwardVersion), bytes(&[1, 4, 0, 0]));
    metadata.insert(Tag::from(TiffTag::Make), Value::Ascii(make));
    metadata.insert(Tag::from(TiffTag::Model), Value::Ascii(model.clone()));
    metadata.insert(
        Tag::from(DngTag::UniqueCameraModel),
        Value::Ascii(format!("Panasonic {model}")),
    );
    if let Some(orientation) = orientation {
        metadata.insert(Tag::from(TiffTag::Orientation), Value::Short(orientation));
    }
    if let Some(serial) = serial {
        metadata.insert(Tag::from(DngTag::CameraSerialNumber), Value::Ascii(serial));
    }
    if let Some(lens) = lens {
        metadata.insert(LENS_MODEL, Value::Ascii(lens));
    }
    metadata.insert(Tag::from(TiffTag::ImageWidth), Value::Unsigned(width));
    metadata.insert(Tag::from(TiffTag::ImageLength), Value::Unsigned(height));
    metadata.insert(Tag::from(TiffTag::BitsPerSample), Value::Short(16));
    metadata.insert(Tag::from(TiffTag::SamplesPerPixel), Value::Short(1));
    metadata.insert(Tag::from(TiffTag::SampleFormat), Value::Short(1));
    metadata.insert(
        Tag::from(TiffTag::PhotometricInterpretation),
        Value::Short(u16::from(PhotometricInterpretation::Cfa)),
    );
    metadata.insert(
        Tag::from(TiffTag::CfaRepeatPatternDim),
        Value::List(vec![Value::Unsigned(2), Value::Unsigned(2)]),
    );
    metadata.insert(
        Tag::from(TiffTag::CfaPattern),
        Value::List(pattern.into_iter().map(Value::Byte).collect()),
    );
    metadata.insert(
        Tag::from(DngTag::CfaPlaneColor),
        Value::List(vec![Value::Byte(0), Value::Byte(1), Value::Byte(2)]),
    );
    metadata.insert(Tag::from(DngTag::CfaLayout), Value::Short(1));
    metadata.insert(
        Tag::from(DngTag::ActiveArea),
        Value::List(vec![
            Value::Unsigned(active.y),
            Value::Unsigned(active.x),
            Value::Unsigned(active.y + active.height),
            Value::Unsigned(active.x + active.width),
        ]),
    );
    metadata.insert(
        Tag::from(DngTag::DefaultCropOrigin),
        Value::List(vec![Value::Unsigned(crop.x), Value::Unsigned(crop.y)]),
    );
    metadata.insert(
        Tag::from(DngTag::DefaultCropSize),
        Value::List(vec![
            Value::Unsigned(crop.width),
            Value::Unsigned(crop.height),
        ]),
    );
    metadata.insert(
        Tag::from(DngTag::BlackLevelRepeatDim),
        Value::List(vec![Value::Short(2), Value::Short(2)]),
    );
    metadata.insert(
        Tag::from(DngTag::BlackLevel),
        Value::List(black_pattern.into_iter().map(Value::Short).collect()),
    );
    metadata.insert(Tag::from(DngTag::WhiteLevel), Value::Unsigned(white));
    if let Some(neutral) = neutral {
        metadata.insert(
            Tag::from(DngTag::AsShotNeutral),
            Value::List(neutral.into_iter().map(rational).collect()),
        );
    }
    Ok(Mapped {
        metadata,
        black_pattern,
    })
}

pub fn process(
    container: &Container,
    metadata: &Directory,
) -> Result<ProcessMetadata, DecodeError> {
    fornax_dng::dng_decoder::process_metadata_from_dng_metadata(
        u32::try_from(container.layout.width)
            .map_err(|_| container::invalid("RW2 width exceeds u32"))?,
        u32::try_from(container.layout.height)
            .map_err(|_| container::invalid("RW2 height exceeds u32"))?,
        metadata,
    )
}

fn active_area(raw: &Directory, width: u32, height: u32) -> Result<Rect, DecodeError> {
    let top = required_u32(raw, ACTIVE_TOP, "active top")?;
    let left = required_u32(raw, ACTIVE_LEFT, "active left")?;
    let bottom = required_u32(raw, ACTIVE_BOTTOM, "active bottom")?;
    let right = required_u32(raw, ACTIVE_RIGHT, "active right")?;
    if left >= right || top >= bottom || right > width || bottom > height {
        return Err(container::invalid(
            "RW2 active-area border coordinates are invalid",
        ));
    }
    Ok(Rect {
        x: left,
        y: top,
        width: right - left,
        height: bottom - top,
    })
}

fn default_crop(raw: &Directory, active: Rect) -> Result<Rect, DecodeError> {
    let Some(top) = optional_u32(raw, CROP_TOP, "crop top")? else {
        return Ok(Rect {
            x: 0,
            y: 0,
            width: active.width,
            height: active.height,
        });
    };
    let left = required_u32(raw, CROP_LEFT, "crop left")?;
    let bottom = required_u32(raw, CROP_BOTTOM, "crop bottom")?;
    let right = required_u32(raw, CROP_RIGHT, "crop right")?;
    if top == 0 && left == 0 && bottom == 0 && right == 0 {
        return Ok(Rect {
            x: 0,
            y: 0,
            width: active.width,
            height: active.height,
        });
    }
    if left >= right || top >= bottom {
        return Err(container::invalid(
            "RW2 default-crop coordinates are invalid",
        ));
    }
    let x = left
        .checked_sub(active.x)
        .ok_or_else(|| container::invalid("RW2 crop starts left of ActiveArea"))?;
    let y = top
        .checked_sub(active.y)
        .ok_or_else(|| container::invalid("RW2 crop starts above ActiveArea"))?;
    let width = right - left;
    let height = bottom - top;
    if x.checked_add(width)
        .is_none_or(|value| value > active.width)
        || y.checked_add(height)
            .is_none_or(|value| value > active.height)
    {
        return Err(container::invalid(
            "RW2 default crop extends outside ActiveArea",
        ));
    }
    Ok(Rect {
        x,
        y,
        width,
        height,
    })
}

fn black_levels(raw: &Directory, container: &Container) -> Result<[u16; 3], DecodeError> {
    let present = [BLACK_RED, BLACK_GREEN, BLACK_BLUE]
        .into_iter()
        .map(|tag| raw.contains_key(&tag))
        .collect::<Vec<_>>();
    if present.iter().all(|present| !present) {
        return Ok([0; 3]);
    }
    if !present.iter().all(|present| *present) {
        return Err(container::invalid(
            "RW2 black-level component tags are incomplete",
        ));
    }
    let offset = if !raw.contains_key(&Tag(0x0118)) || container.version <= 4 {
        15_u16
    } else {
        0
    };
    Ok([
        required_u16(raw, BLACK_RED, "black red")?
            .checked_add(offset)
            .ok_or_else(|| container::invalid("RW2 red black level overflows u16"))?,
        required_u16(raw, BLACK_GREEN, "black green")?
            .checked_add(offset)
            .ok_or_else(|| container::invalid("RW2 green black level overflows u16"))?,
        required_u16(raw, BLACK_BLUE, "black blue")?
            .checked_add(offset)
            .ok_or_else(|| container::invalid("RW2 blue black level overflows u16"))?,
    ])
}

fn spatial_black(black: [u16; 3], pattern: [u8; 4]) -> [u16; 4] {
    pattern.map(|color| black[usize::from(color.min(2))])
}

fn shifted_pattern(pattern: [u8; 4], x: u32, y: u32) -> [u8; 4] {
    std::array::from_fn(|index| {
        let row = index / 2;
        let column = index % 2;
        pattern[((row + y as usize) & 1) * 2 + ((column + x as usize) & 1)]
    })
}

fn white_level(raw: &Directory, bits: u16) -> Result<u32, DecodeError> {
    let values = [LINEARITY_RED, LINEARITY_GREEN, LINEARITY_BLUE]
        .into_iter()
        .filter_map(|tag| {
            unsigned_scalar(raw.get(&tag))
                .and_then(|value| u32::try_from(value).ok())
                .filter(|value| *value != 0)
        })
        .max();
    Ok(values.unwrap_or((1_u32 << bits) - 1))
}

fn neutral(raw: &Directory) -> Result<Option<[f64; 3]>, DecodeError> {
    if let (Some(red), Some(blue)) = (
        container::optional_u16(raw, WB_RED, "white-balance red")?,
        container::optional_u16(raw, WB_BLUE, "white-balance blue")?,
    ) {
        return normalized(red, 256, blue).map(Some);
    }
    if let (Some(red), Some(green), Some(blue)) = (
        container::optional_u16(raw, WB_RED_LEVEL, "white-balance level red")?,
        container::optional_u16(raw, WB_GREEN_LEVEL, "white-balance level green")?,
        container::optional_u16(raw, WB_BLUE_LEVEL, "white-balance level blue")?,
    ) {
        return normalized(red, green, blue).map(Some);
    }
    Ok(None)
}

fn normalized(red: u16, green: u16, blue: u16) -> Result<[f64; 3], DecodeError> {
    if green == 0 {
        return Err(container::invalid(
            "RW2 white-balance green level must not be zero",
        ));
    }
    Ok([
        f64::from(red) / f64::from(green),
        1.0,
        f64::from(blue) / f64::from(green),
    ])
}

fn orientation(raw: &Directory) -> Result<Option<u16>, DecodeError> {
    Ok(
        container::optional_u16(raw, Tag::from(TiffTag::Orientation), "orientation")?
            .filter(|value| (1..=8).contains(value)),
    )
}

fn exif(raw: &Directory, source: &[u8]) -> Result<Option<Directory>, DecodeError> {
    let Some(pointer) = raw
        .get(&EXIF_IFD)
        .and_then(|value| unsigned_scalar(Some(value)))
    else {
        return Ok(None);
    };
    let order = container::byte_order(source)?;
    let directory = fornax_tiff::parse_ifd_at(source, pointer, order, 0)
        .map_err(|error| container::invalid(format!("parsing RW2 Exif IFD failed: {error}")))?;
    Ok(Some(directory))
}

fn required_u16(raw: &Directory, tag: Tag, name: &str) -> Result<u16, DecodeError> {
    container::optional_u16(raw, tag, name)?
        .ok_or_else(|| container::invalid(format!("RW2 {name} is missing")))
}

fn required_u32(raw: &Directory, tag: Tag, name: &str) -> Result<u32, DecodeError> {
    optional_u32(raw, tag, name)?
        .ok_or_else(|| container::invalid(format!("RW2 {name} is missing")))
}

fn optional_u32(raw: &Directory, tag: Tag, name: &str) -> Result<Option<u32>, DecodeError> {
    raw.get(&tag)
        .map(|value| {
            unsigned_scalar(Some(value))
                .and_then(|value| u32::try_from(value).ok())
                .ok_or_else(|| container::invalid(format!("RW2 {name} is invalid")))
        })
        .transpose()
}

fn ascii(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::Ascii(value) if !value.is_empty() => Some(value.clone()),
        _ => None,
    }
}

fn bytes(values: &[u8]) -> Value { Value::List(values.iter().copied().map(Value::Byte).collect()) }

fn rational(value: f64) -> Value {
    const DENOMINATOR: u32 = 1_000_000;
    Value::Rational((value * f64::from(DENOMINATOR)).round() as u32, DENOMINATOR)
}
