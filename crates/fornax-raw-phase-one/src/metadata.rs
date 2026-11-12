use fornax_dng::metadata::DngTag;
use fornax_raw_core::error::DecodeError;
use fornax_raw_core::metadata::ProcessMetadata;
use fornax_tiff::constants::{PhotometricInterpretation, TiffTag};
use fornax_tiff::{Directory, Tag, Value, unsigned_scalar};

use crate::container::{MakerNote, RawLayout};
use crate::invalid;
use crate::profiles::Profile;

pub fn map(
    root: &Directory,
    maker: &MakerNote,
    layout: &RawLayout,
    profile: &Profile,
) -> Result<Directory, DecodeError> {
    let width = u32::try_from(layout.width).map_err(|_| invalid("IIQ width exceeds u32"))?;
    let height = u32::try_from(layout.height).map_err(|_| invalid("IIQ height exceeds u32"))?;
    let active = layout.active_area(maker)?;
    let gains = layout.white_balance(maker)?;
    if gains.iter().any(|gain| !gain.is_finite() || *gain <= 0.0) {
        return Err(invalid(
            "IIQ white-balance gains must be finite and positive",
        ));
    }
    let neutral = [
        f64::from(gains[1]) / f64::from(gains[0]),
        1.0,
        f64::from(gains[1]) / f64::from(gains[2]),
    ];
    let make = root_ascii(root, TiffTag::Make).ok_or_else(|| invalid("IIQ Make is missing"))?;
    let model = root_ascii(root, TiffTag::Model).ok_or_else(|| invalid("IIQ Model is missing"))?;

    let mut directory = Directory::default();
    directory.insert(Tag::from(DngTag::DngVersion), bytes(&[1, 4, 0, 0]));
    directory.insert(Tag::from(DngTag::DngBackwardVersion), bytes(&[1, 4, 0, 0]));
    directory.insert(Tag::from(TiffTag::Make), Value::Ascii(make));
    directory.insert(Tag::from(TiffTag::Model), Value::Ascii(model.clone()));
    directory.insert(
        Tag::from(DngTag::UniqueCameraModel),
        Value::Ascii(format!("{} {model}", profile.make)),
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
        Value::List(vec![Value::Short(2), Value::Short(2)]),
    );
    directory.insert(
        Tag::from(TiffTag::CfaPattern),
        Value::List(profile.cfa.into_iter().map(Value::Byte).collect()),
    );
    directory.insert(
        Tag::from(DngTag::CfaPlaneColor),
        Value::List(vec![Value::Byte(0), Value::Byte(1), Value::Byte(2)]),
    );
    directory.insert(Tag::from(DngTag::CfaLayout), Value::Short(1));
    directory.insert(
        Tag::from(DngTag::ActiveArea),
        Value::List(active.into_iter().map(Value::Unsigned).collect()),
    );
    directory.insert(
        Tag::from(DngTag::DefaultCropOrigin),
        Value::List(vec![Value::Unsigned(0), Value::Unsigned(0)]),
    );
    directory.insert(
        Tag::from(DngTag::DefaultCropSize),
        Value::List(vec![
            Value::Unsigned(active[3] - active[1]),
            Value::Unsigned(active[2] - active[0]),
        ]),
    );
    directory.insert(
        Tag::from(DngTag::BlackLevelRepeatDim),
        Value::List(vec![Value::Short(1), Value::Short(1)]),
    );
    directory.insert(
        Tag::from(DngTag::BlackLevel),
        Value::List(vec![Value::Short(0)]),
    );
    directory.insert(Tag::from(DngTag::WhiteLevel), Value::Unsigned(65_535));
    directory.insert(
        Tag::from(DngTag::AsShotNeutral),
        Value::List(neutral.into_iter().map(rational).collect()),
    );
    if let Some(orientation) = unsigned_scalar(root.get(&Tag::from(TiffTag::Orientation)))
        .and_then(|value| u16::try_from(value).ok())
        .filter(|value| (1..=8).contains(value))
    {
        directory.insert(Tag::from(TiffTag::Orientation), Value::Short(orientation));
    }
    Ok(directory)
}

pub fn process(
    metadata: &Directory,
    width: u32,
    height: u32,
) -> Result<ProcessMetadata, DecodeError> {
    fornax_dng::dng_decoder::process_metadata_from_dng_metadata(width, height, metadata)
}

fn root_ascii(directory: &Directory, tag: TiffTag) -> Option<String> {
    match directory.get(&Tag::from(tag))? {
        Value::Ascii(value) => Some(value.trim_matches('\0').trim().to_owned()),
        _ => None,
    }
}

fn bytes(values: &[u8]) -> Value { Value::List(values.iter().copied().map(Value::Byte).collect()) }

fn rational(value: f64) -> Value {
    const DENOMINATOR: u32 = 1_000_000;
    Value::Rational((value * f64::from(DENOMINATOR)).round() as u32, DENOMINATOR)
}
