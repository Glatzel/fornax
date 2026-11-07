use std::marker::PhantomData;
use std::path::Path;
use std::str::FromStr;

use fornax_core::error::{DecodeError, MetadataValidationError, RawValidationError};
use fornax_core::metadata::{ColorCalibration, Levels, ProcessMetadata};
use fornax_core::protocol::IDecoder;
use fornax_core::{CroppedMosaic, MonoImage, Primitive, RawMosaic};
use fornax_dng::metadata::IDecoderDng;
use fornax_dng::{Directory, DngTag, Value};
use fornax_tiff::Tag;
use fornax_tiff::constants::TiffTag;
use generic_num::num;
use image::math::Rect;
use rawler::decoders::{RawImage, RawImageData, RawPhotometricInterpretation};
use rawler::formats::tiff::Value as RawlerValue;

#[derive(Clone, Debug)]
pub struct RawlerDecoder<T> {
    raw: RawImage,
    _mark: PhantomData<T>,
}

impl<T> IDecoder<&Path, T> for RawlerDecoder<T>
where
    T: Primitive,
{
    fn decode(input: &Path) -> Result<Self, DecodeError> {
        if !input.is_file() {
            return Err(DecodeError::InvalidInput(
                format!("RAW input is not a file: {}", input.display()).into(),
            ));
        }
        let raw = rawler::decode_file(input).map_err(backend_error)?;

        Ok(Self {
            raw,
            _mark: PhantomData,
        })
    }
    fn raw_mosaic(&self) -> Result<RawMosaic, DecodeError> {
        let raw = &self.raw;
        let mosaic = match &raw.data {
            RawImageData::Integer(items) => RawMosaic::U16(
                MonoImage::from_raw(raw.width as u32, raw.height as u32, items.clone())
                    .ok_or_else(|| DecodeError::from(RawValidationError::InvalidImage))?,
            ),
            RawImageData::Float(items) => RawMosaic::F32(
                MonoImage::from_raw(raw.width as u32, raw.height as u32, items.clone())
                    .ok_or_else(|| DecodeError::from(RawValidationError::InvalidImage))?,
            ),
        };
        Ok(mosaic)
    }
    fn cropped_mosaic(&self) -> Result<CroppedMosaic<T>, DecodeError> {
        let raw = &self.raw;
        let cfa = match &raw.photometric {
            RawPhotometricInterpretation::Cfa(config) => &config.cfa,
            RawPhotometricInterpretation::BlackIsZero | RawPhotometricInterpretation::LinearRaw => {
                return Err(DecodeError::Unsupported(
                    "non-CFA RAW payloads are unsupported".into(),
                ));
            }
        };

        if raw.cpp != 1 {
            return Err(DecodeError::Unsupported(
                format!("expected one sample per pixel, got {}", raw.cpp).into(),
            ));
        }
        if cfa.width != 2 || cfa.height != 2 || !cfa.is_rgb() {
            return Err(DecodeError::Unsupported(
                format!("unsupported CFA pattern {cfa}").into(),
            ));
        }

        let width = u32::try_from(raw.width).map_err(|_| RawValidationError::InvalidImage)?;
        let height = u32::try_from(raw.height).map_err(|_| RawValidationError::InvalidImage)?;
        let expected = raw
            .width
            .checked_mul(raw.height)
            .ok_or(RawValidationError::InvalidImage)?;
        let mut cropped = match &raw.data {
            RawImageData::Integer(samples) => {
                if samples.len() != expected {
                    return Err(RawValidationError::InvalidImage.into());
                }
                MonoImage::from_raw(width, height, samples.iter().map(|v| num!(*v)).collect())
                    .ok_or(RawValidationError::InvalidImage)
            }
            RawImageData::Float(samples) => {
                if samples.len() != expected {
                    return Err(RawValidationError::InvalidImage.into());
                }
                if samples.iter().any(|value| !value.is_finite()) {
                    return Err(RawValidationError::NonFiniteSample.into());
                }
                MonoImage::from_raw(width, height, samples.iter().map(|v| num!(*v)).collect())
                    .ok_or(RawValidationError::InvalidImage)
            }
        }?;
        let metadata = self.process_metadata()?;
        cropped.crop_in_place(metadata.active_area);
        Ok(CroppedMosaic::new(cropped))
    }

    fn process_metadata(&self) -> Result<ProcessMetadata, DecodeError> {
        let raw = &self.raw;
        let cfa = match &raw.photometric {
            RawPhotometricInterpretation::Cfa(config) => &config.cfa,
            RawPhotometricInterpretation::BlackIsZero | RawPhotometricInterpretation::LinearRaw => {
                return Err(DecodeError::Unsupported(
                    "non-CFA RAW payloads are
        unsupported"
                        .into(),
                ));
            }
        };

        let active_area = {
            let width = u32::try_from(raw.width).map_err(|_| {
                MetadataValidationError::InvalidGeometry(raw.width.to_string().into())
            })?;
            let height = u32::try_from(raw.height).map_err(|_| {
                MetadataValidationError::InvalidGeometry(raw.height.to_string().into())
            })?;
            if width == 0 || height == 0 {
                return Err(MetadataValidationError::InvalidGeometry(
                    format!("{width}x{height}").into(),
                )
                .into());
            }
            let active_area = raw.active_area.map_or(
                Rect {
                    x: 0,
                    y: 0,
                    width,
                    height,
                },
                |rect| Rect {
                    x: u32::try_from(rect.p.x).unwrap_or(u32::MAX),
                    y: u32::try_from(rect.p.y).unwrap_or(u32::MAX),
                    width: u32::try_from(rect.d.w).unwrap_or(u32::MAX),
                    height: u32::try_from(rect.d.h).unwrap_or(u32::MAX),
                },
            );
            let active_area = align_to_bayer_tile(active_area);
            if active_area.width == 0
                || active_area.height == 0
                || active_area.x.saturating_add(active_area.width) > width
                || active_area.y.saturating_add(active_area.height) > height
            {
                return Err(MetadataValidationError::InvalidActiveArea(
                    format!(
                        "{}x{} at ({}, {}) for {width}x{height}",
                        active_area.width, active_area.height, active_area.x, active_area.y
                    )
                    .into(),
                )
                .into());
            }
            active_area
        };

        let default_crop = match raw.crop_area {
            None => Rect {
                x: 0,
                y: 0,
                width: active_area.width,
                height: active_area.height,
            },
            Some(crop) => {
                let x = u32::try_from(crop.p.x)
                    .ok()
                    .and_then(|x| x.checked_sub(active_area.x));
                let y = u32::try_from(crop.p.y)
                    .ok()
                    .and_then(|y| y.checked_sub(active_area.y));
                let crop_width = u32::try_from(crop.d.w).ok();
                let crop_height = u32::try_from(crop.d.h).ok();
                match (x, y, crop_width, crop_height) {
                    (Some(x), Some(y), Some(width), Some(height))
                        if width > 0
                            && height > 0
                            && x.saturating_add(width) <= active_area.width
                            && y.saturating_add(height) <= active_area.height =>
                    {
                        Rect {
                            x,
                            y,
                            width,
                            height,
                        }
                    }
                    _ => {
                        return Err(MetadataValidationError::InvalidDefaultCrop(
                            format!(
                                "{}x{} at ({}, {}) outside active area {}x{} at ({}, {})",
                                crop.d.w,
                                crop.d.h,
                                crop.p.x,
                                crop.p.y,
                                active_area.width,
                                active_area.height,
                                active_area.x,
                                active_area.y
                            )
                            .into(),
                        )
                        .into());
                    }
                }
            }
        };

        let black = {
            let black_width = u32::try_from(raw.blacklevel.width).map_err(|_| {
                MetadataValidationError::InvalidLevels("black-level width exceeds u32".into())
            })?;
            let black_height = u32::try_from(raw.blacklevel.height).map_err(|_| {
                MetadataValidationError::InvalidLevels("black-level height exceeds u32".into())
            })?;
            let black_values = raw.blacklevel.as_vec();
            let expected_black = (black_width as usize)
                .checked_mul(black_height as usize)
                .ok_or_else(|| {
                    MetadataValidationError::InvalidLevels("black-level shape overflows".into())
                })?;
            if black_width == 0
                || black_height == 0
                || black_values.len() != expected_black
                || black_values.iter().any(|value| !value.is_finite())
            {
                return Err(MetadataValidationError::InvalidLevels(
                    format!(
                        "black-level shape is {black_width}x{black_height} with {} samples",
                        black_values.len()
                    )
                    .into(),
                )
                .into());
            }
            MonoImage::from_raw(
                black_width,
                black_height,
                black_values.into_iter().map(f64::from).collect(),
            )
            .ok_or_else(|| {
                MetadataValidationError::InvalidLevels("invalid black-level image".into())
            })?
        };

        let white = {
            let white_value = raw.whitelevel.0.first().copied().ok_or_else(|| {
                MetadataValidationError::InvalidLevels("white level is missing".into())
            })?;
            let white = f64::from(white_value);
            if white <= 0.0 {
                return Err(MetadataValidationError::InvalidLevels(
                    "white level must be greater than zero".into(),
                )
                .into());
            }
            white
        };

        let bayer_pattern = fornax_core::BayerPattern::from_str(&cfa.name)
            .map_err(|_| DecodeError::Unsupported(format!("unsupported CFA pattern {cfa}").into()))?
            .shifted(active_area.x, active_area.y);
        let mut raw_color = fornax_dng::raw_color_metadata(&synthesize_dng_metadata(raw))?;
        let has_dng_as_shot_neutral = raw.dng_tags.contains_key(&u16::from(DngTag::AsShotNeutral));
        let valid_as_shot_neutral = raw_color.as_shot_neutral.is_some_and(|neutral| {
            neutral
                .iter()
                .all(|value| value.is_finite() && *value > 0.0)
        });
        if !has_dng_as_shot_neutral || !valid_as_shot_neutral {
            raw_color.as_shot_neutral = None;
            let neutral = raw.neutralwb();
            if neutral[..3]
                .iter()
                .all(|value| value.is_finite() && *value > 0.0)
            {
                raw_color.as_shot_neutral = Some([
                    f64::from(neutral[0]),
                    f64::from(neutral[1]),
                    f64::from(neutral[2]),
                ]);
            } else {
                let [r, g, b, _] = raw.wb_coeffs;
                if [r, g, b]
                    .iter()
                    .all(|value| value.is_finite() && *value > 0.0)
                {
                    raw_color.maker_gains = Some((f64::from(r / g), f64::from(b / g)));
                }
            }
        }
        let color_calibration = ColorCalibration::new(raw_color);

        Ok(ProcessMetadata {
            active_area,
            default_crop,
            levels: Levels {
                black,
                black_delta_h: vec![],
                black_delta_v: vec![],
                white,
            },
            bayer_pattern,
            color_calibration,
        })
    }
}
impl<T> IDecoderDng<&Path, T> for RawlerDecoder<T>
where
    T: Primitive,
{
    fn dng_metadata(&self) -> Result<Directory, DecodeError> {
        Ok(synthesize_dng_metadata(&self.raw))
    }
}

fn synthesize_dng_metadata(raw: &RawImage) -> Directory {
    let mut metadata = Directory::default();
    let width = u32::try_from(raw.width).unwrap_or(u32::MAX);
    let height = u32::try_from(raw.height).unwrap_or(u32::MAX);
    metadata.insert(
        DngTag::DngVersion.into(),
        Value::List(vec![
            Value::Byte(1),
            Value::Byte(4),
            Value::Byte(0),
            Value::Byte(0),
        ]),
    );
    metadata.insert(Tag::from(TiffTag::ImageWidth), Value::Unsigned(width));
    metadata.insert(Tag::from(TiffTag::ImageLength), Value::Unsigned(height));
    metadata.insert(
        TiffTag::BitsPerSample.into(),
        Value::Short(u16::try_from(raw.bps).unwrap_or(u16::MAX)),
    );
    metadata.insert(
        TiffTag::SamplesPerPixel.into(),
        Value::Short(u16::try_from(raw.cpp).unwrap_or(u16::MAX)),
    );

    let active_area = raw.active_area.map_or_else(
        || Rect {
            x: 0,
            y: 0,
            width,
            height,
        },
        |rect| Rect {
            x: u32::try_from(rect.x()).unwrap_or(u32::MAX),
            y: u32::try_from(rect.y()).unwrap_or(u32::MAX),
            width: u32::try_from(rect.width()).unwrap_or(u32::MAX),
            height: u32::try_from(rect.height()).unwrap_or(u32::MAX),
        },
    );
    let active_area = align_to_bayer_tile(active_area);
    metadata.insert(
        DngTag::ActiveArea.into(),
        unsigned_values([
            active_area.y,
            active_area.x,
            active_area.y.saturating_add(active_area.height),
            active_area.x.saturating_add(active_area.width),
        ]),
    );
    let (crop_x, crop_y, crop_width, crop_height) =
        raw.crop_area
            .map_or((0, 0, active_area.width, active_area.height), |crop| {
                (
                    u32::try_from(crop.x())
                        .unwrap_or(u32::MAX)
                        .saturating_sub(active_area.x),
                    u32::try_from(crop.y())
                        .unwrap_or(u32::MAX)
                        .saturating_sub(active_area.y),
                    u32::try_from(crop.width()).unwrap_or(u32::MAX),
                    u32::try_from(crop.height()).unwrap_or(u32::MAX),
                )
            });
    metadata.insert(
        Tag::from(DngTag::DefaultCropOrigin),
        unsigned_values([crop_x, crop_y]),
    );
    metadata.insert(
        DngTag::DefaultCropSize.into(),
        unsigned_values([crop_width, crop_height]),
    );

    let black_width = u32::try_from(raw.blacklevel.width).unwrap_or(u32::MAX);
    let black_height = u32::try_from(raw.blacklevel.height).unwrap_or(u32::MAX);
    metadata.insert(
        DngTag::BlackLevelRepeatDim.into(),
        unsigned_values([black_width, black_height]),
    );
    metadata.insert(
        DngTag::BlackLevel.into(),
        rational_values(raw.blacklevel.as_vec().into_iter().map(f64::from)),
    );
    metadata.insert(
        DngTag::WhiteLevel.into(),
        unsigned_values(raw.whitelevel.0.iter().copied()),
    );

    if let RawPhotometricInterpretation::Cfa(config) = &raw.photometric {
        metadata.insert(
            TiffTag::CfaRepeatPatternDim.into(),
            unsigned_values([
                u32::try_from(config.cfa.width).unwrap_or(u32::MAX),
                u32::try_from(config.cfa.height).unwrap_or(u32::MAX),
            ]),
        );
        metadata.insert(
            Tag::from(TiffTag::CfaPattern),
            byte_values(config.cfa.flat_pattern()),
        );
        metadata.insert(
            Tag::from(DngTag::CfaPlaneColor),
            unsigned_values([0_u32, 1, 2]),
        );
        metadata.insert(Tag::from(DngTag::CfaLayout), Value::Short(1));
    }

    if !raw.make.is_empty() {
        metadata.insert(Tag::from(TiffTag::Make), Value::Ascii(raw.make.clone()));
    }
    if !raw.model.is_empty() {
        metadata.insert(Tag::from(TiffTag::Model), Value::Ascii(raw.model.clone()));
    }
    let unique_model = format!("{} {}", raw.clean_make, raw.clean_model)
        .trim()
        .to_owned();
    if !unique_model.is_empty() {
        metadata.insert(
            Tag::from(DngTag::UniqueCameraModel),
            Value::Ascii(unique_model),
        );
    }
    if raw.wb_coeffs[..3]
        .iter()
        .all(|value| value.is_finite() && *value > 0.0)
    {
        metadata.insert(
            DngTag::AsShotNeutral.into(),
            rational_values([
                1.0 / f64::from(raw.wb_coeffs[0]),
                1.0,
                1.0 / f64::from(raw.wb_coeffs[2]),
            ]),
        );
    }
    if let Some(matrix) = raw.color_matrix.values().next()
        && matrix.len() == 9
    {
        metadata.insert(Tag::from(DngTag::CalibrationIlluminant1), Value::Short(21));
        metadata.insert(
            DngTag::ColorMatrix1.into(),
            rational_values(matrix.iter().copied().map(f64::from)),
        );
    }

    metadata.extend(
        raw.dng_tags
            .iter()
            .map(|(&tag, value)| (Tag(tag), dng_value(value))),
    );
    metadata
}

fn unsigned_values(values: impl IntoIterator<Item = u32>) -> Value {
    dng_values(values.into_iter().map(Value::Unsigned).collect())
}

fn byte_values(values: Vec<u8>) -> Value {
    dng_values(values.into_iter().map(Value::Byte).collect())
}

fn rational_values(values: impl IntoIterator<Item = f64>) -> Value {
    dng_values(
        values
            .into_iter()
            .map(|value| {
                let numerator = (value * 1_000_000.0).round();
                let numerator = numerator.clamp(f64::from(i32::MIN), f64::from(i32::MAX)) as i32;
                Value::SRational(numerator, 1_000_000)
            })
            .collect(),
    )
}

fn backend_error(error: rawler::RawlerError) -> DecodeError {
    match error {
        rawler::RawlerError::Unsupported {
            what,
            model,
            make,
            mode,
        } => DecodeError::Unsupported(
            format!("{what}; make={make}, model={model}, mode={mode}").into(),
        ),
        rawler::RawlerError::DecoderFailed(message) => DecodeError::Backend(message.into()),
    }
}

fn dng_value(value: &RawlerValue) -> Value {
    match value {
        RawlerValue::Byte(values)
        | RawlerValue::Undefined(values)
        | RawlerValue::Unknown(_, values) => {
            dng_values(values.iter().copied().map(Value::Byte).collect())
        }
        RawlerValue::Ascii(value) => Value::Ascii(value.first().cloned().unwrap_or_default()),
        RawlerValue::Short(values) => {
            dng_values(values.iter().copied().map(Value::Short).collect())
        }
        RawlerValue::Long(values) => {
            dng_values(values.iter().copied().map(Value::Unsigned).collect())
        }
        RawlerValue::Rational(values) => dng_values(
            values
                .iter()
                .map(|value| Value::Rational(value.n, value.d))
                .collect(),
        ),
        RawlerValue::SByte(values) => {
            dng_values(values.iter().copied().map(Value::SignedByte).collect())
        }
        RawlerValue::SShort(values) => {
            dng_values(values.iter().copied().map(Value::SignedShort).collect())
        }
        RawlerValue::SLong(values) => {
            dng_values(values.iter().copied().map(Value::Signed).collect())
        }
        RawlerValue::SRational(values) => dng_values(
            values
                .iter()
                .map(|value| Value::SRational(value.n, value.d))
                .collect(),
        ),
        RawlerValue::Float(values) => {
            dng_values(values.iter().copied().map(Value::Float).collect())
        }
        RawlerValue::Double(values) => {
            dng_values(values.iter().copied().map(Value::Double).collect())
        }
    }
}

fn dng_values(values: Vec<Value>) -> Value {
    match values.as_slice() {
        [value] => value.clone(),
        _ => Value::List(values),
    }
}

const fn align_to_bayer_tile(mut area: Rect) -> Rect {
    if area.x & 1 == 1 {
        area.x = area.x.saturating_add(1);
        area.width = area.width.saturating_sub(1);
    }
    if area.y & 1 == 1 {
        area.y = area.y.saturating_add(1);
        area.height = area.height.saturating_sub(1);
    }
    area.width &= !1;
    area.height &= !1;
    area
}

#[cfg(test)]
mod tests {
    use fornax_core::metadata::WhiteSource;
    use fornax_dng::{DngTag, Value};
    use rawler::CFA;
    use rawler::decoders::Camera;
    use rawler::pixarray::PixU16;
    use rawler::rawimage::{CFAConfig, WhiteLevel};

    use super::*;

    fn configured_raw() -> RawImage {
        let camera = Camera {
            cfa: CFA::new("RGGB"),
            active_area: Some([0, 0, 1, 1]),
            blacklevel: Some(vec![0; 4]),
            whitelevel: Some(vec![65_535]),
            ..Camera::default()
        };
        let photometric = RawPhotometricInterpretation::Cfa(CFAConfig::new_from_camera(&camera));
        RawImage::new(
            camera,
            PixU16::new_with((0..16).collect(), 4, 4),
            1,
            [1.0, 1.0, 1.0, f32::NAN],
            photometric,
            None,
            Some(WhiteLevel(vec![65_535])),
            false,
        )
    }

    #[test]
    fn cropped_mosaic_returns_only_the_active_area() {
        let decoder: RawlerDecoder<f32> = RawlerDecoder {
            raw: configured_raw(),
            _mark: PhantomData,
        };

        let cropped = decoder.cropped_mosaic().expect("active area should crop");

        assert_eq!(cropped.image().dimensions(), (2, 2));
        assert_eq!(cropped.image().get_pixel(0, 0), &image::Luma([0.0]));
        assert_eq!(cropped.image().get_pixel(1, 0), &image::Luma([1.0]));
        assert_eq!(cropped.image().get_pixel(0, 1), &image::Luma([4.0]));
        assert_eq!(cropped.image().get_pixel(1, 1), &image::Luma([5.0]));
    }

    #[test]
    fn dng_metadata_synthesizes_decoded_geometry_levels_and_cfa() {
        let decoder: RawlerDecoder<f32> = RawlerDecoder {
            raw: configured_raw(),
            _mark: PhantomData,
        };

        let metadata = decoder
            .dng_metadata()
            .expect("metadata should be available");

        assert_eq!(
            metadata.get(&Tag::from(TiffTag::ImageWidth)),
            Some(&Value::Unsigned(4))
        );
        assert_eq!(
            metadata.get(&Tag::from(TiffTag::ImageLength)),
            Some(&Value::Unsigned(4))
        );
        assert_eq!(
            metadata.get(&Tag::from(DngTag::ActiveArea)),
            Some(&Value::List(vec![
                Value::Unsigned(0),
                Value::Unsigned(0),
                Value::Unsigned(2),
                Value::Unsigned(2),
            ]))
        );
        assert_eq!(
            metadata.get(&Tag::from(TiffTag::CfaRepeatPatternDim)),
            Some(&Value::List(vec![Value::Unsigned(2), Value::Unsigned(2),]))
        );
        assert_eq!(
            metadata.get(&Tag::from(TiffTag::CfaPattern)),
            Some(&Value::List(vec![
                Value::Byte(0),
                Value::Byte(1),
                Value::Byte(1),
                Value::Byte(2),
            ]))
        );
        assert_eq!(
            metadata.get(&Tag::from(DngTag::WhiteLevel)),
            Some(&Value::Unsigned(65_535))
        );
    }

    #[test]
    fn process_metadata_uses_rawler_dng_color_tags() {
        let mut raw = configured_raw();
        raw.dng_tags.extend([
            (
                u16::from(DngTag::AsShotNeutral),
                RawlerValue::Float(vec![0.5, 1.0, 0.25]),
            ),
            (
                u16::from(DngTag::CalibrationIlluminant1),
                RawlerValue::Short(vec![u16::from(fornax_core::metadata::LightSource::D65)]),
            ),
            (
                u16::from(DngTag::ColorMatrix1),
                RawlerValue::Float(vec![1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]),
            ),
        ]);
        let decoder: RawlerDecoder<f32> = RawlerDecoder {
            raw,
            _mark: PhantomData,
        };

        let metadata = decoder
            .process_metadata()
            .expect("process metadata should be available");

        assert_eq!(
            metadata.color_calibration.as_shot_source(),
            WhiteSource::AsShotNeutral
        );
        assert!(!metadata.color_calibration.is_fallback());
    }
}
