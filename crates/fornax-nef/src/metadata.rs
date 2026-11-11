use fornax_dng::metadata::DngTag;
use fornax_raw_core::error::DecodeError;
use fornax_raw_core::metadata::{MetadataSource, ProcessMetadata};
use fornax_tiff::constants::TiffTag;
use fornax_tiff::{Directory, ParsedTiff, Tag, Value};

use crate::container::RawLayout;
use crate::makernote::MakerNote;
use crate::wb;

const CROP_AREA: Tag = Tag(0x0045);
const BLACK_LEVEL: Tag = Tag(0x003d);

#[derive(Clone, Debug)]
pub(crate) struct MappedMetadata {
    pub directory: Directory,
    pub wb_source: MetadataSource,
    pub black_source: MetadataSource,
}

pub(crate) fn map(
    parsed: &ParsedTiff,
    raw_ifd: usize,
    layout: &RawLayout,
    maker: &MakerNote,
) -> Result<MappedMetadata, DecodeError> {
    let raw = parsed
        .directories
        .get(raw_ifd)
        .ok_or_else(|| invalid("selected Nikon raw IFD is missing"))?;
    let mut directory = parsed.root.clone();
    directory.extend(raw.clone());
    let width = u32::try_from(layout.width).map_err(|_| invalid("Nikon width exceeds u32"))?;
    let height = u32::try_from(layout.height).map_err(|_| invalid("Nikon height exceeds u32"))?;
    let (active_top, active_left, active_width, active_height) =
        active_area(maker.directory(), layout)?;
    let (neutral, wb_source) = wb::neutral(maker.directory())?;
    let (black, repeat, black_source) = black_levels(maker.directory(), layout)?;
    let white = (1_u32 << layout_bits(layout)) - 1;
    let cfa = cfa_pattern(raw)?;

    directory.insert(
        Tag::from(DngTag::DngVersion),
        Value::List(vec![
            Value::Byte(1),
            Value::Byte(4),
            Value::Byte(0),
            Value::Byte(0),
        ]),
    );
    directory.insert(
        Tag::from(DngTag::DngBackwardVersion),
        Value::List(vec![
            Value::Byte(1),
            Value::Byte(4),
            Value::Byte(0),
            Value::Byte(0),
        ]),
    );
    directory.insert(Tag::from(TiffTag::ImageWidth), Value::Unsigned(width));
    directory.insert(Tag::from(TiffTag::ImageLength), Value::Unsigned(height));
    directory.insert(Tag::from(TiffTag::BitsPerSample), Value::Short(16));
    directory.insert(Tag::from(TiffTag::SamplesPerPixel), Value::Short(1));
    directory.insert(Tag::from(TiffTag::SampleFormat), Value::Short(1));
    directory.insert(
        Tag::from(TiffTag::PhotometricInterpretation),
        Value::Short(u16::from(
            fornax_tiff::constants::PhotometricInterpretation::Cfa,
        )),
    );
    directory.insert(
        Tag::from(TiffTag::CfaRepeatPatternDim),
        Value::List(vec![Value::Short(2), Value::Short(2)]),
    );
    directory.insert(
        Tag::from(TiffTag::CfaPattern),
        Value::List(cfa.into_iter().map(Value::Byte).collect()),
    );
    directory.insert(
        Tag::from(DngTag::CfaPlaneColor),
        Value::List(vec![Value::Byte(0), Value::Byte(1), Value::Byte(2)]),
    );
    directory.insert(Tag::from(DngTag::CfaLayout), Value::Short(1));
    directory.insert(
        Tag::from(DngTag::ActiveArea),
        Value::List(vec![
            Value::Unsigned(
                u32::try_from(active_top).map_err(|_| invalid("ActiveArea exceeds u32"))?,
            ),
            Value::Unsigned(
                u32::try_from(active_left).map_err(|_| invalid("ActiveArea exceeds u32"))?,
            ),
            Value::Unsigned(
                u32::try_from(active_top + active_height)
                    .map_err(|_| invalid("ActiveArea exceeds u32"))?,
            ),
            Value::Unsigned(
                u32::try_from(active_left + active_width)
                    .map_err(|_| invalid("ActiveArea exceeds u32"))?,
            ),
        ]),
    );
    directory.insert(
        Tag::from(DngTag::DefaultCropOrigin),
        Value::List(vec![Value::Unsigned(0), Value::Unsigned(0)]),
    );
    directory.insert(
        Tag::from(DngTag::DefaultCropSize),
        Value::List(vec![
            Value::Unsigned(
                u32::try_from(active_width).map_err(|_| invalid("DefaultCropSize exceeds u32"))?,
            ),
            Value::Unsigned(
                u32::try_from(active_height).map_err(|_| invalid("DefaultCropSize exceeds u32"))?,
            ),
        ]),
    );
    directory.insert(
        Tag::from(DngTag::BlackLevelRepeatDim),
        Value::List(repeat.into_iter().map(Value::Short).collect()),
    );
    directory.insert(
        Tag::from(DngTag::BlackLevel),
        Value::List(black.into_iter().map(Value::Short).collect()),
    );
    directory.insert(Tag::from(DngTag::WhiteLevel), Value::Unsigned(white));
    if let Some(neutral) = neutral {
        directory.insert(
            Tag::from(DngTag::AsShotNeutral),
            Value::List(neutral.into_iter().map(rational).collect()),
        );
    }
    if !directory.contains_key(&Tag::from(DngTag::UniqueCameraModel))
        && let Some(Value::Ascii(model)) = directory.get(&Tag::from(TiffTag::Model))
    {
        directory.insert(
            Tag::from(DngTag::UniqueCameraModel),
            Value::Ascii(format!("Nikon {model}")),
        );
    }
    Ok(MappedMetadata {
        directory,
        wb_source,
        black_source,
    })
}

pub(crate) fn process(
    metadata: &MappedMetadata,
    width: u32,
    height: u32,
) -> Result<ProcessMetadata, DecodeError> {
    let mut process = fornax_dng::dng_decoder::process_metadata_from_dng_metadata(
        width,
        height,
        &metadata.directory,
    )?;
    process.wb_source = metadata.wb_source;
    process.black_source = metadata.black_source;
    Ok(process)
}

fn active_area(
    maker: &Directory,
    layout: &RawLayout,
) -> Result<(usize, usize, usize, usize), DecodeError> {
    let Some(value) = maker.get(&CROP_AREA) else {
        return Ok((0, 0, layout.width, layout.height));
    };
    let values = unsigned_values(value)?;
    let [left, top, width, height]: [usize; 4] = values
        .try_into()
        .map_err(|_| invalid("Nikon CropArea must contain four values"))?;
    if width == 0
        || height == 0
        || left.saturating_add(width) > layout.width
        || top.saturating_add(height) > layout.height
    {
        return Err(invalid(format!(
            "Nikon CropArea {width}x{height} at ({left}, {top}) is outside {}x{}",
            layout.width, layout.height
        )));
    }
    Ok((top, left, width, height))
}

fn black_levels(
    maker: &Directory,
    layout: &RawLayout,
) -> Result<(Vec<u16>, [u16; 2], MetadataSource), DecodeError> {
    let Some(value) = maker.get(&BLACK_LEVEL) else {
        return Ok((vec![0], [1, 1], MetadataSource::Default));
    };
    let mut values = unsigned_values(value)?
        .into_iter()
        .map(u16::try_from)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| invalid("Nikon black level exceeds u16"))?;
    if values.len() != 4 {
        return Err(invalid("Nikon black level must contain four values"));
    }
    if layout_bits(layout) == 12 {
        for value in &mut values {
            *value >>= 2;
        }
    }
    Ok((
        values,
        [2, 2],
        MetadataSource::Tag {
            tag: BLACK_LEVEL.0,
            version: None,
        },
    ))
}

fn cfa_pattern(raw: &Directory) -> Result<[u8; 4], DecodeError> {
    let values = raw
        .get(&Tag::from(TiffTag::CfaPattern))
        .ok_or_else(|| invalid("Nikon raw IFD has no CFAPattern"))
        .and_then(unsigned_values)?;
    values
        .into_iter()
        .map(|value| u8::try_from(value).map_err(|_| invalid("Nikon CFA plane exceeds u8")))
        .collect::<Result<Vec<_>, _>>()?
        .try_into()
        .map_err(|_| invalid("Nikon CFAPattern must contain four values"))
}

fn unsigned_values(value: &Value) -> Result<Vec<usize>, DecodeError> {
    fn one(value: &Value) -> Result<usize, DecodeError> {
        match value {
            Value::Byte(value) => Ok(usize::from(*value)),
            Value::Short(value) => Ok(usize::from(*value)),
            Value::Unsigned(value) => {
                usize::try_from(*value).map_err(|_| invalid("Nikon value exceeds usize"))
            }
            _ => Err(invalid("Nikon metadata value is not unsigned")),
        }
    }
    match value {
        Value::List(values) => values.iter().map(one).collect(),
        value => Ok(vec![one(value)?]),
    }
}

fn layout_bits(layout: &RawLayout) -> u32 {
    use crate::container::LayoutKind;
    match layout.kind {
        LayoutKind::Packed { bits, .. } | LayoutKind::UnpackedWords { bits, .. } => u32::from(bits),
        LayoutKind::StripedPacked12 => 12,
    }
}

fn rational(value: f64) -> Value {
    const DENOMINATOR: u32 = 1_000_000;
    Value::Rational((value * f64::from(DENOMINATOR)).round() as u32, DENOMINATOR)
}

fn invalid(detail: impl Into<Box<str>>) -> DecodeError { DecodeError::InvalidInput(detail.into()) }
