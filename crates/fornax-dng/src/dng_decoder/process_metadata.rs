use fornax_raw_core::error::DecodeError;
use fornax_raw_core::metadata::{
    ColorCalibration, Levels, LightSource, Matrix3x3, MetadataSource, ProcessMetadata,
    RawColorMeta, RawProfile,
};
use fornax_raw_core::{BayerChannel, BayerPattern, MonoImage, Primitive};
use fornax_tiff::constants::TiffTag;
use fornax_tiff::{DirectoryLookup, Tag, Value};
use generic_num::num;
use image::math::Rect;

use crate::metadata::{Directory, DngTag};

pub fn process_metadata_from_dng_metadata(
    full_width: u32,
    full_height: u32,
    dng_metadata: &Directory,
) -> Result<ProcessMetadata, DecodeError> {
    from_lookup(full_width, full_height, dng_metadata)
}

pub(super) fn from_lookup<D>(
    full_width: u32,
    full_height: u32,
    dng_metadata: &D,
) -> Result<ProcessMetadata, DecodeError>
where
    D: DirectoryLookup + ?Sized,
{
    let active_area = active_area(dng_metadata, full_width, full_height)?;
    let default_crop = default_crop(dng_metadata, active_area)?;
    let black = black_levels(dng_metadata)?;
    let black_delta_h = scalar_values(dng_metadata.get(&Tag::from(DngTag::BlackLevelDeltaH)))?
        .into_iter()
        .map(cast_number)
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| metadata_error("BlackLevelDeltaH does not fit the output type"))?;
    let black_delta_v = scalar_values(dng_metadata.get(&Tag::from(DngTag::BlackLevelDeltaV)))?
        .into_iter()
        .map(cast_number)
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| metadata_error("BlackLevelDeltaV does not fit the output type"))?;
    let white = scalar_number(dng_metadata.get(&Tag::from(DngTag::WhiteLevel)))?
        .and_then(cast_number)
        .ok_or_else(|| metadata_error("WhiteLevel is missing or invalid"))?;
    let bayer_pattern = bayer_pattern(dng_metadata, active_area)?;

    let color_calibration = ColorCalibration::new(raw_color_metadata(dng_metadata)?);
    let wb_source = if dng_metadata.contains_key(&Tag::from(DngTag::AsShotNeutral)) {
        MetadataSource::Tag {
            tag: u16::from(DngTag::AsShotNeutral),
            version: None,
        }
    } else if dng_metadata.contains_key(&Tag::from(DngTag::AsShotWhiteXY)) {
        MetadataSource::Tag {
            tag: u16::from(DngTag::AsShotWhiteXY),
            version: None,
        }
    } else {
        MetadataSource::Default
    };
    let black_source = if dng_metadata.contains_key(&Tag::from(DngTag::BlackLevel)) {
        MetadataSource::Tag {
            tag: u16::from(DngTag::BlackLevel),
            version: None,
        }
    } else {
        MetadataSource::Default
    };
    Ok(ProcessMetadata {
        active_area,
        default_crop,
        levels: Levels {
            black,
            black_delta_h,
            black_delta_v,
            white,
        },
        bayer_pattern,
        color_calibration,
        wb_source,
        black_source,
    })
}

/// Maps raw DNG color tags into the decoder-facing color model.
fn raw_color_metadata<D>(dng_metadata: &D) -> Result<RawColorMeta, DecodeError>
where
    D: DirectoryLookup + ?Sized,
{
    let mut profiles = vec![
        color_profile(
            dng_metadata,
            DngTag::ColorMatrix1,
            DngTag::ForwardMatrix1,
            DngTag::CameraCalibration1,
            DngTag::CalibrationIlluminant1,
        )?,
        color_profile(
            dng_metadata,
            DngTag::ColorMatrix2,
            DngTag::ForwardMatrix2,
            DngTag::CameraCalibration2,
            DngTag::CalibrationIlluminant2,
        )?,
        color_profile(
            dng_metadata,
            DngTag::ColorMatrix3,
            DngTag::ForwardMatrix3,
            DngTag::CameraCalibration3,
            DngTag::CalibrationIlluminant3,
        )?,
    ];
    while profiles.last().is_some_and(profile_is_empty) {
        profiles.pop();
    }
    Ok(RawColorMeta {
        analog_balance: color_vector(dng_metadata, DngTag::AnalogBalance)?,
        as_shot_neutral: color_vector(dng_metadata, DngTag::AsShotNeutral)?,
        as_shot_white_xy: color_vector(dng_metadata, DngTag::AsShotWhiteXY)?,
        maker_gains: None,
        profiles,
    })
}

fn color_profile<D>(
    metadata: &D,
    color_matrix: DngTag,
    forward_matrix: DngTag,
    camera_calibration: DngTag,
    illuminant: DngTag,
) -> Result<RawProfile, DecodeError>
where
    D: DirectoryLookup + ?Sized,
{
    Ok(RawProfile {
        illuminant: light_source(metadata, illuminant)?,
        illuminant_data: None,
        color_matrix: matrix(metadata, color_matrix)?,
        forward_matrix: matrix(metadata, forward_matrix)?,
        camera_calibration: matrix(metadata, camera_calibration)?,
    })
}

const fn profile_is_empty(profile: &RawProfile) -> bool {
    profile.illuminant.is_none()
        && profile.illuminant_data.is_none()
        && profile.color_matrix.is_none()
        && profile.forward_matrix.is_none()
        && profile.camera_calibration.is_none()
}

fn color_vector<const N: usize, D>(
    metadata: &D,
    tag: DngTag,
) -> Result<Option<[f64; N]>, DecodeError>
where
    D: DirectoryLookup + ?Sized,
{
    let values = scalar_values::<f64>(metadata.get(&Tag::from(tag)))?;
    if values.is_empty() {
        return Ok(None);
    }
    values.try_into().map(Some).map_err(|values: Vec<f64>| {
        metadata_error(format!(
            "{tag} must contain {N} values, found {}",
            values.len()
        ))
    })
}

fn light_source<D>(metadata: &D, tag: DngTag) -> Result<Option<LightSource>, DecodeError>
where
    D: DirectoryLookup + ?Sized,
{
    let Some(value) =
        unsigned_values(metadata.get(&Tag::from(tag)))?.and_then(|values| values.first().copied())
    else {
        return Ok(None);
    };
    let value =
        u16::try_from(value).map_err(|_| metadata_error(format!("{tag} is out of range")))?;
    Ok(Some(LightSource::from(value)))
}

fn matrix<D>(metadata: &D, tag: DngTag) -> Result<Option<Matrix3x3>, DecodeError>
where
    D: DirectoryLookup + ?Sized,
{
    let Some(value) = metadata.get(&Tag::from(tag)) else {
        return Ok(None);
    };
    let values = scalar_values::<f64>(Some(value))?;
    let values: [f64; 9] = values.try_into().map_err(|_| {
        metadata_error(format!(
            "{tag} must contain nine values, found {}",
            value.count()
        ))
    })?;
    Ok(Some([
        [values[0], values[1], values[2]],
        [values[3], values[4], values[5]],
        [values[6], values[7], values[8]],
    ]))
}

fn active_area<D>(metadata: &D, full_width: u32, full_height: u32) -> Result<Rect, DecodeError>
where
    D: DirectoryLookup + ?Sized,
{
    let Some(values) = unsigned_values(metadata.get(&Tag::from(DngTag::ActiveArea)))? else {
        return Ok(Rect {
            x: 0,
            y: 0,
            width: full_width,
            height: full_height,
        });
    };
    let [top, left, bottom, right]: [u32; 4] = values
        .try_into()
        .map_err(|_| DecodeError::Backend("ActiveArea must contain four values".into()))?;
    if top >= bottom || left >= right || bottom > full_height || right > full_width {
        return Err(DecodeError::Backend(format!(
            "ActiveArea ({top}, {left}, {bottom}, {right}) is outside {full_width}x{full_height}"
        ).into()));
    }
    Ok(Rect {
        x: left,
        y: top,
        width: right - left,
        height: bottom - top,
    })
}
fn default_crop<D>(metadata: &D, active: Rect) -> Result<Rect, DecodeError>
where
    D: DirectoryLookup + ?Sized,
{
    let origin = unsigned_values(metadata.get(&Tag::from(DngTag::DefaultCropOrigin)))?;
    let size = unsigned_values(metadata.get(&Tag::from(DngTag::DefaultCropSize)))?;
    let (Some(origin), Some(size)) = (origin, size) else {
        return Ok(Rect {
            x: 0,
            y: 0,
            width: active.width,
            height: active.height,
        });
    };
    let [x, y]: [u32; 2] = origin
        .try_into()
        .map_err(|_| metadata_error("DefaultCropOrigin must contain two values"))?;
    let [width, height]: [u32; 2] = size
        .try_into()
        .map_err(|_| metadata_error("DefaultCropSize must contain two values"))?;
    if width == 0
        || height == 0
        || x.saturating_add(width) > active.width
        || y.saturating_add(height) > active.height
    {
        return Err(metadata_error(format!(
            "DefaultCrop {width}x{height} at ({x}, {y}) is outside active area {}x{}",
            active.width, active.height
        )));
    }
    Ok(Rect {
        x,
        y,
        width,
        height,
    })
}

fn black_levels<D, T>(metadata: &D) -> Result<MonoImage<T>, DecodeError>
where
    D: DirectoryLookup + ?Sized,
    T: Primitive,
{
    let repeat = unsigned_values(metadata.get(&Tag::from(DngTag::BlackLevelRepeatDim)))?
        .unwrap_or_else(|| vec![1, 1]);
    let [width, height]: [u32; 2] = repeat
        .try_into()
        .map_err(|_| metadata_error("BlackLevelRepeatDim must contain two values"))?;
    let levels = scalar_values(metadata.get(&Tag::from(DngTag::BlackLevel)))?
        .into_iter()
        .map(cast_number)
        .collect::<Option<Vec<T>>>()
        .ok_or_else(|| metadata_error("BlackLevel is missing or invalid"))?;
    let width_usize = usize::try_from(width)
        .map_err(|_| metadata_error("BlackLevelRepeatDim exceeds the platform address space"))?;
    let height_usize = usize::try_from(height)
        .map_err(|_| metadata_error("BlackLevelRepeatDim exceeds the platform address space"))?;
    let expected = width_usize
        .checked_mul(height_usize)
        .ok_or_else(|| metadata_error("BlackLevelRepeatDim exceeds the platform address space"))?;
    if width == 0 || height == 0 || levels.len() != expected {
        return Err(metadata_error(format!(
            "BlackLevel contains {} values for repeat dimensions {width}x{height}",
            levels.len()
        )));
    }
    MonoImage::from_raw(width, height, levels)
        .ok_or_else(|| metadata_error("black-level image dimensions are invalid"))
}

fn bayer_pattern<D>(metadata: &D, active: Rect) -> Result<BayerPattern, DecodeError>
where
    D: DirectoryLookup + ?Sized,
{
    let repeat = unsigned_values(metadata.get(&Tag::from(TiffTag::CfaRepeatPatternDim)))?
        .ok_or_else(|| metadata_error("CFARepeatPatternDim is missing"))?;
    let [width, height]: [u32; 2] = repeat
        .try_into()
        .map_err(|_| metadata_error("CFARepeatPatternDim must contain two values"))?;
    if width != 2 || height != 2 {
        return Err(DecodeError::Unsupported(
            format!("CFA layout {width}x{height} is not conventional 2x2 Bayer").into(),
        ));
    }
    let mut plane_colors = scalar_values(metadata.get(&Tag::from(DngTag::CfaPlaneColor)))?;
    if plane_colors.is_empty() {
        plane_colors = vec![0.0, 1.0, 2.0];
    }
    if plane_colors != [0.0, 1.0, 2.0] {
        return Err(DecodeError::Unsupported(
            "CFA plane colors are not RGB".into(),
        ));
    }
    if unsigned_values(metadata.get(&Tag::from(DngTag::CfaLayout)))?
        .and_then(|values| values.first().copied())
        .is_some_and(|layout| layout != 1)
    {
        return Err(DecodeError::Unsupported(
            "non-rectangular CFA layouts are unsupported".into(),
        ));
    }
    let pattern = unsigned_values(metadata.get(&Tag::from(TiffTag::CfaPattern)))?
        .ok_or_else(|| metadata_error("CFAPattern is missing"))?;
    if pattern.len() != 4 {
        return Err(metadata_error(format!(
            "CFAPattern contains {} values, expected 4",
            pattern.len()
        )));
    }

    let color_at = |x: u32, y: u32| -> Result<u32, DecodeError> {
        let pattern_x = (x + active.x) & 1;
        let pattern_y = (y + active.y) & 1;
        let color = pattern[(pattern_y * 2 + pattern_x) as usize];
        match color {
            0..=2 => Ok(color),
            _ => Err(metadata_error(format!(
                "CFAPattern contains unknown plane {color}"
            ))),
        }
    };
    let channel = |x: u32, y: u32| -> Result<BayerChannel, DecodeError> {
        match color_at(x, y)? {
            0 => Ok(BayerChannel::R),
            2 => Ok(BayerChannel::B),
            1 => {
                let row_has_red = color_at(0, y)? == 0 || color_at(1, y)? == 0;
                Ok(if row_has_red {
                    BayerChannel::GRedRow
                } else {
                    BayerChannel::GBlueRow
                })
            }
            _ => unreachable!("the color value is validated by color_at"),
        }
    };

    let mask = [
        channel(0, 0)?,
        channel(1, 0)?,
        channel(0, 1)?,
        channel(1, 1)?,
    ];
    BayerPattern::ALL
        .into_iter()
        .find(|pattern| pattern.as_mask() == &mask)
        .ok_or_else(|| DecodeError::Unsupported("unsupported RGB Bayer pattern".into()))
        .map(|p| p.shifted(active.x, active.y))
}

fn scalar_number<T>(value: Option<&Value>) -> Result<Option<T>, DecodeError>
where
    T: Primitive,
{
    match value {
        None => Ok(None),
        Some(Value::Byte(value)) => Ok(Some(num!(*value))),
        Some(Value::Bytes(values)) => values
            .first()
            .map_or(Ok(None), |value| Ok(Some(num!(*value)))),
        Some(Value::SignedByte(value)) => Ok(Some(num!(*value))),
        Some(Value::Short(value)) => Ok(Some(num!(*value))),
        Some(Value::SignedShort(value)) => Ok(Some(num!(*value))),
        Some(Value::Unsigned(value)) => Ok(Some(num!(*value))),
        Some(Value::Signed(value)) => Ok(Some(num!(*value))),
        Some(Value::UnsignedBig(value)) => Ok(Some(num!(*value))),
        Some(Value::SignedBig(value)) => Ok(Some(num!(*value))),
        Some(Value::Float(value)) => Ok(Some(num!(*value))),
        Some(Value::Double(value)) => Ok(Some(num!(*value))),
        Some(Value::Rational(numerator, denominator)) => {
            Ok(Some(num!(*numerator) / num!(*denominator)))
        }
        Some(Value::SRational(numerator, denominator)) => {
            Ok(Some(num!(*numerator) / num!(*denominator)))
        }
        Some(
            Value::RationalBig(numerator, denominator)
            | Value::SRationalBig(numerator, denominator),
        ) => Ok(Some(num!(*numerator) / num!(*denominator))),
        Some(Value::List(values)) => values
            .first()
            .map_or(Ok(None), |value| scalar_number(Some(value))),
        _ => Err(metadata_error("tag value is not numeric")),
    }
}

fn scalar_values<T>(value: Option<&Value>) -> Result<Vec<T>, DecodeError>
where
    T: Primitive,
{
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    match value {
        Value::Bytes(values) => values.iter().map(|value| Ok(num!(*value))).collect(),
        Value::List(values) => values
            .iter()
            .map(|value| {
                scalar_number(Some(value))?
                    .ok_or_else(|| metadata_error("list tag contains a non-numeric value"))
            })
            .collect(),
        _ => Ok(scalar_number(Some(value))?.into_iter().collect()),
    }
}

fn unsigned_values(value: Option<&Value>) -> Result<Option<Vec<u32>>, DecodeError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let values = match value {
        Value::Bytes(values) => values
            .iter()
            .map(|value| Ok(u32::from(*value)))
            .collect::<Result<Vec<_>, DecodeError>>()?,
        Value::List(values) => values
            .iter()
            .map(unsigned_scalar)
            .collect::<Result<Vec<_>, _>>()?,
        value => vec![unsigned_scalar(value)?],
    };
    Ok(Some(values))
}

fn cast_number<T>(value: f64) -> Option<T>
where
    T: Primitive,
{
    num_traits::cast(value)
}

fn metadata_error(detail: impl Into<Box<str>>) -> DecodeError {
    DecodeError::Backend(detail.into())
}

fn unsigned_scalar(value: &Value) -> Result<u32, DecodeError> {
    let number = match value {
        Value::Byte(value) => *value as u32,
        Value::Bytes(values) => {
            return values
                .first()
                .copied()
                .map(u32::from)
                .ok_or_else(|| DecodeError::Backend("byte tag value is empty".into()));
        }
        Value::Short(value) => *value as u32,
        Value::Unsigned(value) | Value::Ifd(value) => *value,
        Value::UnsignedBig(value) => u32::try_from(*value)
            .map_err(|_| DecodeError::Backend("tag value does not fit u32".into()))?,
        Value::IfdBig(value) => u32::try_from(*value)
            .map_err(|_| DecodeError::Backend("IFD pointer does not fit u32".into()))?,
        Value::Rational(numerator, denominator) if *denominator == 1 => *numerator,
        _ => {
            return Err(DecodeError::Backend(
                "tag does not contain unsigned integers".into(),
            ));
        }
    };
    Ok(number)
}
#[cfg(test)]
mod tests {
    use fornax_raw_core::metadata::{LightSource, WhiteSource};

    use super::*;

    fn matrix_values(matrix: [[f64; 3]; 3]) -> Value {
        Value::List(
            matrix
                .into_iter()
                .flatten()
                .map(|value| Value::SRational((value * 1_000_000.0) as i32, 1_000_000))
                .collect(),
        )
    }

    fn valid_metadata() -> Directory {
        let mut metadata = Directory::default();
        metadata.insert(Tag::from(DngTag::BlackLevel), Value::Short(0));
        metadata.insert(Tag::from(DngTag::WhiteLevel), Value::Short(1));
        metadata.insert(
            Tag::from(TiffTag::CfaRepeatPatternDim),
            Value::List(vec![Value::Short(2), Value::Short(2)]),
        );
        metadata.insert(
            Tag::from(TiffTag::CfaPattern),
            Value::List(vec![
                Value::Byte(0),
                Value::Byte(1),
                Value::Byte(1),
                Value::Byte(2),
            ]),
        );
        metadata
    }

    #[test]
    fn raw_color_metadata_maps_all_dng_color_profiles() {
        let mut metadata = valid_metadata();
        let color1 = [[1.0, 0.0, 0.0], [0.0, 2.0, 0.0], [0.0, 0.0, 3.0]];
        let color2 = [[4.0, 0.0, 0.0], [0.0, 5.0, 0.0], [0.0, 0.0, 6.0]];
        let color3 = [[13.0, 0.0, 0.0], [0.0, 14.0, 0.0], [0.0, 0.0, 15.0]];
        let forward1 = [[7.0, 0.0, 0.0], [0.0, 8.0, 0.0], [0.0, 0.0, 9.0]];
        let forward2 = [[10.0, 0.0, 0.0], [0.0, 11.0, 0.0], [0.0, 0.0, 12.0]];
        let forward3 = [[16.0, 0.0, 0.0], [0.0, 17.0, 0.0], [0.0, 0.0, 18.0]];
        metadata.insert(DngTag::AnalogBalance.into(), vector_values([2.0, 1.0, 0.5]));
        metadata.insert(
            DngTag::AsShotNeutral.into(),
            vector_values([0.5, 1.0, 0.25]),
        );
        metadata.insert(DngTag::AsShotWhiteXY.into(), vector_values([0.3127, 0.329]));
        metadata.insert(Tag::from(DngTag::ColorMatrix1), matrix_values(color1));
        metadata.insert(Tag::from(DngTag::ColorMatrix2), matrix_values(color2));
        metadata.insert(Tag::from(DngTag::ColorMatrix3), matrix_values(color3));
        metadata.insert(Tag::from(DngTag::ForwardMatrix1), matrix_values(forward1));
        metadata.insert(Tag::from(DngTag::ForwardMatrix2), matrix_values(forward2));
        metadata.insert(Tag::from(DngTag::ForwardMatrix3), matrix_values(forward3));
        metadata.insert(
            Tag::from(DngTag::CalibrationIlluminant1),
            Value::Short(u16::from(LightSource::StandardLightA)),
        );
        metadata.insert(
            Tag::from(DngTag::CalibrationIlluminant2),
            Value::Short(u16::from(LightSource::D65)),
        );
        metadata.insert(
            Tag::from(DngTag::CalibrationIlluminant3),
            Value::Short(u16::from(LightSource::D75)),
        );

        let raw = raw_color_metadata(&metadata).unwrap();

        assert_eq!(raw.analog_balance, Some([2.0, 1.0, 0.5]));
        assert_eq!(raw.as_shot_neutral, Some([0.5, 1.0, 0.25]));
        assert_eq!(raw.as_shot_white_xy, Some([0.3127, 0.329]));
        assert_eq!(raw.profiles.len(), 3);
        assert_eq!(
            raw.profiles[0].illuminant,
            Some(LightSource::StandardLightA)
        );
        assert_eq!(raw.profiles[1].illuminant, Some(LightSource::D65));
        assert_eq!(raw.profiles[2].illuminant, Some(LightSource::D75));
        assert_eq!(raw.profiles[0].color_matrix, Some(color1));
        assert_eq!(raw.profiles[1].color_matrix, Some(color2));
        assert_eq!(raw.profiles[2].color_matrix, Some(color3));
        assert_eq!(raw.profiles[0].forward_matrix, Some(forward1));
        assert_eq!(raw.profiles[1].forward_matrix, Some(forward2));
        assert_eq!(raw.profiles[2].forward_matrix, Some(forward3));
    }

    #[test]
    fn raw_color_metadata_preserves_camera_calibrations() {
        let mut metadata = valid_metadata();
        let calibration1 = [[1.0, 0.1, 0.0], [0.0, 2.0, 0.0], [0.0, 0.0, 3.0]];
        let calibration2 = [[4.0, 0.0, 0.0], [0.1, 5.0, 0.0], [0.0, 0.0, 6.0]];
        metadata.insert(
            Tag::from(DngTag::CameraCalibration1),
            matrix_values(calibration1),
        );
        metadata.insert(
            Tag::from(DngTag::CameraCalibration2),
            matrix_values(calibration2),
        );

        let raw = raw_color_metadata(&metadata).unwrap();

        assert_eq!(raw.profiles[0].camera_calibration, Some(calibration1));
        assert_eq!(raw.profiles[1].camera_calibration, Some(calibration2));
    }

    #[test]
    fn absent_profiles_are_not_synthesized() {
        let raw = raw_color_metadata(&valid_metadata()).unwrap();

        assert!(raw.profiles.is_empty());
    }

    #[test]
    fn process_metadata_uses_as_shot_neutral() {
        let mut metadata = valid_metadata();
        metadata.insert(
            DngTag::AsShotNeutral.into(),
            vector_values([0.5, 1.0, 0.25]),
        );

        let process = process_metadata_from_dng_metadata(2, 2, &metadata).unwrap();

        assert_eq!(
            process.color_calibration.as_shot_source(),
            WhiteSource::AsShotNeutral
        );
    }

    #[test]
    fn malformed_color_matrix_returns_metadata_error() {
        let mut metadata = valid_metadata();
        metadata.insert(
            Tag::from(DngTag::ColorMatrix1),
            Value::List(vec![Value::Short(1), Value::Short(2)]),
        );

        assert!(process_metadata_from_dng_metadata(2, 2, &metadata).is_err());
    }

    fn vector_values(values: impl IntoIterator<Item = f64>) -> Value {
        Value::List(
            values
                .into_iter()
                .map(|value| Value::SRational((value * 1_000_000.0) as i32, 1_000_000))
                .collect(),
        )
    }
}
