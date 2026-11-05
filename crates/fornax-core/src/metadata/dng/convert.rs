// fn metadata_from_dng(
//     pixels: &MosaicData,
//     dng_metadata: &DngMetadata,
// ) -> Result<DngMetadata, DecodeError> {
//     let (full_width, full_height) = pixels.dimensions();
//     let active_area = active_area(dng_metadata, full_width, full_height)?;
//     let default_crop = default_crop(dng_metadata, active_area)?;
//     let black = black_levels(dng_metadata)?;
//     let black_delta_h =
// scalar_values(dng_metadata.get(&DngTag::BlackLevelDeltaH))?
//         .into_iter()
//         .map(cast_number)
//         .collect::<Option<Vec<T>>>()
//         .ok_or_else(|| metadata_error("BlackLevelDeltaH does not fit the
// output type"))?;     let black_delta_v =
// scalar_values(dng_metadata.get(&DngTag::BlackLevelDeltaV))?
//         .into_iter()
//         .map(cast_number)
//         .collect::<Option<Vec<T>>>()
//         .ok_or_else(|| metadata_error("BlackLevelDeltaV does not fit the
// output type"))?;     let white =
// scalar_number(dng_metadata.get(&DngTag::WhiteLevel))?
//         .and_then(cast_number)
//         .ok_or_else(|| metadata_error("WhiteLevel is missing or invalid"))?;
//     let bayer_pattern = bayer_pattern(dng_metadata, active_area)?;
//     let white_balance = white_balance(dng_metadata)?;

use image::math::Rect;

use crate::error::DecodeError;
use crate::metadata::dng::{DngMetadata, DngTag};

//     Ok(ProcessMetadata {
//         active_area,
//         default_crop,
//         levels: Levels {
//             black,
//             black_delta_h,
//             black_delta_v,
//             white,
//         },
//         bayer_pattern,
//         white_balance,
//         dng_metadata: dng_metadata.clone(),
//     })
// }



// fn default_crop(metadata: &DngMetadata, active: Rect) -> Result<Rect,
// DecodeError> {     let origin =
// unsigned_values(metadata.get(&DngTag::DefaultCropOrigin))?
//         .ok_or_else(|| metadata_error("DefaultCropOrigin is missing"))?;
//     let size = unsigned_values(metadata.get(&DngTag::DefaultCropSize))?
//         .ok_or_else(|| metadata_error("DefaultCropSize is missing"))?;
//     let [x, y]: [u32; 2] = origin
//         .try_into()
//         .map_err(|_| metadata_error("DefaultCropOrigin must contain two
// values"))?;     let [width, height]: [u32; 2] = size
//         .try_into()
//         .map_err(|_| metadata_error("DefaultCropSize must contain two
// values"))?;     if width == 0
//         || height == 0
//         || x.saturating_add(width) > active.width
//         || y.saturating_add(height) > active.height
//     {
//         return Err(metadata_error(format!(
//             "DefaultCrop {width}x{height} at ({x}, {y}) is outside active
// area {}x{}",             active.width, active.height
//         )));
//     }
//     Ok(Rect {
//         x,
//         y,
//         width,
//         height,
//     })
// }

// fn black_levels<T>(metadata: &DngMetadata) -> Result<MonoImage<T>,
// DecodeError> where
//     T: Primitive,
// {
//     let repeat =
//         unsigned_values(metadata.get(&DngTag::BlackLevelRepeatDim))?.
// unwrap_or_else(|| vec![1, 1]);     let [width, height]: [u32; 2] = repeat
//         .try_into()
//         .map_err(|_| metadata_error("BlackLevelRepeatDim must contain two
// values"))?;     let levels =
// scalar_values(metadata.get(&DngTag::BlackLevel))?         .into_iter()
//         .map(cast_number)
//         .collect::<Option<Vec<T>>>()
//         .ok_or_else(|| metadata_error("BlackLevel is missing or invalid"))?;
//     let width_usize = usize::try_from(width)
//         .map_err(|_| metadata_error("BlackLevelRepeatDim exceeds the platform
// address space"))?;     let height_usize = usize::try_from(height)
//         .map_err(|_| metadata_error("BlackLevelRepeatDim exceeds the platform
// address space"))?;     let expected = width_usize
//         .checked_mul(height_usize)
//         .ok_or_else(|| metadata_error("BlackLevelRepeatDim exceeds the
// platform address space"))?;     if width == 0 || height == 0 || levels.len()
// != expected {         return Err(metadata_error(format!(
//             "BlackLevel contains {} values for repeat dimensions
// {width}x{height}",             levels.len()
//         )));
//     }
//     MonoImage::from_raw(width, height, levels)
//         .ok_or_else(|| metadata_error("black-level image dimensions are
// invalid")) }

// fn bayer_pattern(metadata: &DngMetadata, active: Rect) ->
// Result<BayerPattern, DecodeError> {     let repeat =
// unsigned_values(metadata.get(&DngTag::CfaRepeatPatternDim))?
//         .ok_or_else(|| metadata_error("CFARepeatPatternDim is missing"))?;
//     let [width, height]: [u32; 2] = repeat
//         .try_into()
//         .map_err(|_| metadata_error("CFARepeatPatternDim must contain two
// values"))?;     if width != 2 || height != 2 {
//         return Err(DecodeError::Unsupported(
//             format!("CFA layout {width}x{height} is not conventional 2x2
// Bayer").into(),         ));
//     }
//     let plane_colors =
//         scalar_values(metadata.get(&DngTag::CfaPlaneColor)).
// unwrap_or_else(|_| vec![0.0, 1.0, 2.0]);     if plane_colors != [0.0, 1.0,
// 2.0] {         return Err(DecodeError::Unsupported(
//             "CFA plane colors are not RGB".into(),
//         ));
//     }
//     if unsigned_values(metadata.get(&DngTag::CfaLayout))?
//         .and_then(|values| values.first().copied())
//         .is_some_and(|layout| layout != 1)
//     {
//         return Err(DecodeError::Unsupported(
//             "non-rectangular CFA layouts are unsupported".into(),
//         ));
//     }
//     let pattern = unsigned_values(metadata.get(&DngTag::CfaPattern))?
//         .ok_or_else(|| metadata_error("CFAPattern is missing"))?;
//     if pattern.len() != 4 {
//         return Err(metadata_error(format!(
//             "CFAPattern contains {} values, expected 4",
//             pattern.len()
//         )));
//     }

//     let color_at = |x: u32, y: u32| -> Result<u32, DecodeError> {
//         let pattern_x = (x + active.x) & 1;
//         let pattern_y = (y + active.y) & 1;
//         let color = pattern[(pattern_y * 2 + pattern_x) as usize];
//         match color {
//             0..=2 => Ok(color),
//             _ => Err(metadata_error(format!(
//                 "CFAPattern contains unknown plane {color}"
//             ))),
//         }
//     };
//     let channel = |x: u32, y: u32| -> Result<BayerChannel, DecodeError> {
//         match color_at(x, y)? {
//             0 => Ok(BayerChannel::R),
//             2 => Ok(BayerChannel::B),
//             1 => {
//                 let row_has_red = color_at(0, y)? == 0 || color_at(1, y)? ==
// 0;                 Ok(if row_has_red {
//                     BayerChannel::GRedRow
//                 } else {
//                     BayerChannel::GBlueRow
//                 })
//             }
//             _ => unreachable!("the color value is validated by color_at"),
//         }
//     };

//     let mask = [
//         channel(0, 0)?,
//         channel(1, 0)?,
//         channel(0, 1)?,
//         channel(1, 1)?,
//     ];
//     BayerPattern::ALL
//         .into_iter()
//         .find(|pattern| pattern.as_mask() == &mask)
//         .ok_or_else(|| DecodeError::Unsupported("unsupported RGB Bayer
// pattern".into())) }

// fn white_balance<T>(metadata: &DngMetadata) ->
// Result<Option<WhiteBalance<T>>, DecodeError> where
//     T: Primitive,
// {
//     if metadata.get(&DngTag::AsShotNeutral).is_none() {
//         return Ok(None);
//     }
//     let values = scalar_values(metadata.get(&DngTag::AsShotNeutral))?;
//     let [red, green, blue]: [f64; 3] = values
//         .try_into()
//         .map_err(|_| metadata_error("AsShotNeutral must contain three
// values"))?;     if red <= 0.0 || green <= 0.0 || blue <= 0.0 {
//         return Err(metadata_error("AsShotNeutral values must be positive"));
//     }
//     let (Some(red), Some(green), Some(blue)) =
//         (cast_number(red), cast_number(green), cast_number(blue))
//     else {
//         return Err(metadata_error("AsShotNeutral does not fit the output
// type"));     };
//     Ok(Some(WhiteBalance::AsShotNeutral(red, green, blue)))
// }

// fn scalar_number(value: Option<&Value>) -> Result<Option<f64>, DecodeError> {
//     match value {
//         None => Ok(None),
//         Some(Value::Byte(value)) => Ok(Some(*value as f64)),
//         Some(Value::SignedByte(value)) => Ok(Some(*value as f64)),
//         Some(Value::Short(value)) => Ok(Some(*value as f64)),
//         Some(Value::SignedShort(value)) => Ok(Some(*value as f64)),
//         Some(Value::Unsigned(value)) => Ok(Some(*value as f64)),
//         Some(Value::Signed(value)) => Ok(Some(*value as f64)),
//         Some(Value::UnsignedBig(value)) => Ok(Some(*value as f64)),
//         Some(Value::SignedBig(value)) => Ok(Some(*value as f64)),
//         Some(Value::Float(value)) => Ok(Some(*value as f64)),
//         Some(Value::Double(value)) => Ok(Some(*value)),
//         Some(Value::Rational(numerator, denominator)) => rational(*numerator,
// *denominator),         Some(Value::SRational(numerator, denominator)) =>
// rational(*numerator, *denominator),         #[expect(deprecated, reason =
// "tiff exposes legacy BigTIFF rational variants")]
//         Some(Value::RationalBig(numerator, denominator)) =>
// rational(*numerator, *denominator),         #[expect(deprecated, reason =
// "tiff exposes legacy BigTIFF rational variants")]
//         Some(Value::SRationalBig(numerator, denominator)) =>
// rational(*numerator, *denominator),         Some(Value::List(values)) =>
// values             .first()
//             .map_or(Ok(None), |value| scalar_number(Some(value))),
//         _ => Err(metadata_error("tag value is not numeric")),
//     }
// }

// fn rational<T>(numerator: T, denominator: T) -> Result<Option<f64>,
// DecodeError> where
//     T: ToPrimitive,
// {
//     let numerator = numerator
//         .to_f64()
//         .ok_or_else(|| metadata_error("rational numerator is too large"))?;
//     let denominator = denominator
//         .to_f64()
//         .ok_or_else(|| metadata_error("rational denominator is too large"))?;
//     if denominator == 0.0 {
//         return Err(metadata_error("rational denominator is zero"));
//     }
//     Ok(Some(numerator / denominator))
// }

// fn scalar_values(value: Option<&Value>) -> Result<Vec<f64>, DecodeError> {
//     let Some(value) = value else {
//         return Ok(Vec::new());
//     };
//     match value {
//         Value::List(values) => values
//             .iter()
//             .map(|value| {
//                 scalar_number(Some(value))?
//                     .ok_or_else(|| metadata_error("list tag contains a
// non-numeric value"))             })
//             .collect(),
//         _ => Ok(scalar_number(Some(value))?.into_iter().collect()),
//     }
// }

// fn unsigned_values(value: Option<&Value>) -> Result<Option<Vec<u32>>,
// DecodeError> {     let Some(value) = value else {
//         return Ok(None);
//     };
//     let values = match value {
//         Value::List(values) => values
//             .iter()
//             .map(unsigned_scalar)
//             .collect::<Result<Vec<_>, _>>()?,
//         value => vec![unsigned_scalar(value)?],
//     };
//     Ok(Some(values))
// }



// fn cast_number<T>(value: f64) -> Option<T>
// where
//     T: Primitive,
// {
//     num_traits::cast(value)
// }

// fn metadata_error(detail: impl Into<Box<str>>) -> DecodeError {
//     DecodeError::Backend(detail.into())
// }
