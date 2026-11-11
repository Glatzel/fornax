use fornax_dng::metadata::DngTag;
use fornax_raw_core::error::DecodeError;
use fornax_raw_core::metadata::ProcessMetadata;
use fornax_tiff::constants::{PhotometricInterpretation, TiffTag};
use fornax_tiff::{Directory, Tag, Value, unsigned_vec};

use crate::container::Container;

const EXPOSURE_TIME: Tag = Tag(0x829a);
const FNUMBER: Tag = Tag(0x829d);
const PHOTOGRAPHIC_SENSITIVITY: Tag = Tag(0x8827);
const DATE_TIME_ORIGINAL: Tag = Tag(0x9003);
const SUB_SEC_TIME_ORIGINAL: Tag = Tag(0x9291);
const FLASH: Tag = Tag(0x9209);
const FOCAL_LENGTH: Tag = Tag(0x920a);
const BODY_SERIAL_NUMBER: Tag = Tag(0xa431);
const LENS_MAKE: Tag = Tag(0xa433);
const LENS_MODEL: Tag = Tag(0xa434);

const SAFE_EXIF_TAGS: &[Tag] = &[
    EXPOSURE_TIME,
    FNUMBER,
    PHOTOGRAPHIC_SENSITIVITY,
    DATE_TIME_ORIGINAL,
    SUB_SEC_TIME_ORIGINAL,
    FLASH,
    FOCAL_LENGTH,
    BODY_SERIAL_NUMBER,
    LENS_MAKE,
    LENS_MODEL,
];

pub(crate) fn map(container: &Container) -> Result<Directory, DecodeError> {
    let mut directory = Directory::default();
    copy_preserved(&mut directory, &container.root);
    copy_preserved(&mut directory, &container.raw);
    for tag in SAFE_EXIF_TAGS {
        if let Some(value) = container.exif.get(tag) {
            directory.insert(*tag, value.clone());
        }
    }
    directory.insert(
        Tag::from(TiffTag::Make),
        Value::Ascii(container.make.clone()),
    );
    directory.insert(
        Tag::from(TiffTag::Model),
        Value::Ascii(container.model.clone()),
    );
    directory.insert(
        Tag::from(DngTag::UniqueCameraModel),
        Value::Ascii(container.unique_model.clone()),
    );
    directory.insert(Tag::from(DngTag::DngVersion), bytes(&[1, 4, 0, 0]));
    directory.insert(Tag::from(DngTag::DngBackwardVersion), bytes(&[1, 4, 0, 0]));
    directory.insert(
        Tag::from(TiffTag::ImageWidth),
        Value::Unsigned(
            u32::try_from(container.layout.width).map_err(|_| invalid("3FR width exceeds u32"))?,
        ),
    );
    directory.insert(
        Tag::from(TiffTag::ImageLength),
        Value::Unsigned(
            u32::try_from(container.layout.height)
                .map_err(|_| invalid("3FR height exceeds u32"))?,
        ),
    );
    directory.insert(Tag::from(TiffTag::BitsPerSample), Value::Short(16));
    directory.insert(Tag::from(TiffTag::SamplesPerPixel), Value::Short(1));
    directory.insert(Tag::from(TiffTag::SampleFormat), Value::Short(1));
    directory.insert(
        Tag::from(TiffTag::PhotometricInterpretation),
        Value::Short(u16::from(PhotometricInterpretation::Cfa)),
    );
    directory.insert(
        Tag::from(TiffTag::CfaRepeatPatternDim),
        Value::List(
            container
                .cfa
                .repeat
                .into_iter()
                .map(Value::Unsigned)
                .collect(),
        ),
    );
    directory.insert(
        Tag::from(TiffTag::CfaPattern),
        Value::List(
            container
                .cfa
                .pattern
                .iter()
                .copied()
                .map(Value::Byte)
                .collect(),
        ),
    );
    directory.insert(
        Tag::from(DngTag::CfaPlaneColor),
        Value::List(
            container
                .cfa
                .plane_colors
                .iter()
                .copied()
                .map(Value::Byte)
                .collect(),
        ),
    );
    directory.insert(
        Tag::from(DngTag::CfaLayout),
        Value::Short(container.cfa.layout),
    );

    let active = container.active_area;
    directory.insert(
        Tag::from(DngTag::ActiveArea),
        Value::List(vec![
            Value::Unsigned(active.y),
            Value::Unsigned(active.x),
            Value::Unsigned(active.y + active.height),
            Value::Unsigned(active.x + active.width),
        ]),
    );
    let (origin, size) = default_crop(container)?;
    directory.insert(
        Tag::from(DngTag::DefaultCropOrigin),
        Value::List(vec![Value::Unsigned(origin[0]), Value::Unsigned(origin[1])]),
    );
    directory.insert(
        Tag::from(DngTag::DefaultCropSize),
        Value::List(vec![Value::Unsigned(size[0]), Value::Unsigned(size[1])]),
    );
    directory.insert(
        Tag::from(DngTag::BlackLevelRepeatDim),
        Value::List(vec![Value::Short(1), Value::Short(1)]),
    );
    directory
        .entry(Tag::from(DngTag::BlackLevel))
        .or_insert(Value::Short(0));
    if !directory.contains_key(&Tag::from(DngTag::WhiteLevel)) {
        return Err(invalid("3FR raw IFD has no WhiteLevel"));
    }
    Ok(directory)
}

pub(crate) fn process(
    metadata: &Directory,
    width: u32,
    height: u32,
) -> Result<ProcessMetadata, DecodeError> {
    fornax_dng::dng_decoder::process_metadata_from_dng_metadata(width, height, metadata)
}

fn copy_preserved(directory: &mut Directory, raw: &Directory) {
    for (tag, value) in raw {
        if !structural(*tag) {
            directory.insert(*tag, value.clone());
        }
    }
}

fn structural(tag: Tag) -> bool {
    matches!(
        TiffTag::from(tag),
        TiffTag::NewSubfileType
            | TiffTag::ImageWidth
            | TiffTag::ImageLength
            | TiffTag::BitsPerSample
            | TiffTag::Compression
            | TiffTag::PhotometricInterpretation
            | TiffTag::StripOffsets
            | TiffTag::SamplesPerPixel
            | TiffTag::RowsPerStrip
            | TiffTag::StripByteCounts
            | TiffTag::PlanarConfiguration
            | TiffTag::Predictor
            | TiffTag::TileWidth
            | TiffTag::TileLength
            | TiffTag::TileOffsets
            | TiffTag::TileByteCounts
            | TiffTag::SampleFormat
            | TiffTag::SubIfds
            | TiffTag::ExifIfdPointer
    )
}

fn default_crop(container: &Container) -> Result<([u32; 2], [u32; 2]), DecodeError> {
    let active = container.active_area;
    let origin = if container
        .raw
        .contains_key(&Tag::from(DngTag::DefaultCropOrigin))
    {
        unsigned_vec::<_, u32>(&container.raw, DngTag::DefaultCropOrigin)
            .map_err(|error| invalid(format!("3FR DefaultCropOrigin is invalid: {error}")))?
            .try_into()
            .map_err(|_| invalid("3FR DefaultCropOrigin must contain two values"))?
    } else {
        [active.x, active.y]
    };
    let size = if container
        .raw
        .contains_key(&Tag::from(DngTag::DefaultCropSize))
    {
        unsigned_vec::<_, u32>(&container.raw, DngTag::DefaultCropSize)
            .map_err(|error| invalid(format!("3FR DefaultCropSize is invalid: {error}")))?
            .try_into()
            .map_err(|_| invalid("3FR DefaultCropSize must contain two values"))?
    } else {
        [active.width, active.height]
    };
    let x = origin[0]
        .checked_sub(active.x)
        .ok_or_else(|| invalid("3FR default crop starts left of ActiveArea"))?;
    let y = origin[1]
        .checked_sub(active.y)
        .ok_or_else(|| invalid("3FR default crop starts above ActiveArea"))?;
    if size[0] == 0
        || size[1] == 0
        || x.saturating_add(size[0]) > active.width
        || y.saturating_add(size[1]) > active.height
    {
        return Err(invalid("3FR default crop is outside ActiveArea"));
    }
    Ok(([x, y], size))
}

fn bytes(values: &[u8]) -> Value { Value::List(values.iter().copied().map(Value::Byte).collect()) }

fn invalid(detail: impl Into<Box<str>>) -> DecodeError { DecodeError::InvalidInput(detail.into()) }
