use std::path::Path;
use std::str::FromStr;

use fornax_core::error::{DecodeError, MetadataValidationError, RawValidationError};
use fornax_core::metadata::dng::DngMetadata;
use fornax_core::metadata::process::{Levels, ProcessMetadata};
use fornax_core::protocol::IDecoder;
use fornax_core::{BayerPattern, CroppedMosaic, MonoImage, RawMosaic, Primitive};
use generic_num::num;
use image::math::Rect;
use rawler::decoders::{RawImage, RawImageData, RawPhotometricInterpretation};

#[derive(Clone, Copy, Debug, Default)]
pub struct RawlerDecoder;

impl<P, T> IDecoder<P, T> for RawlerDecoder
where
    T: Primitive,
    P: AsRef<Path>,
{
    fn decode(
        &self,
        input: P,
    ) -> Result<(RawMosaic, DngMetadata, ProcessMetadata<T>), DecodeError> {
        let input = input.as_ref();
        if !input.is_file() {
            return Err(DecodeError::InvalidInput(
                format!("RAW input is not a file: {}", input.display()).into(),
            ));
        }
        let raw = rawler::decode_file(input).map_err(backend_error)?;
        let mosaic = match &raw.data {
            RawImageData::Integer(items) => RawMosaic::U16(
                MonoImage::from_raw(raw.width as _, raw.height as _, items.clone())
                    .ok_or_else(|| DecodeError::from(RawValidationError::InvalidImage))?,
            ),
            RawImageData::Float(items) => RawMosaic::F32(
                MonoImage::from_raw(raw.width as _, raw.height as _, items.clone())
                    .ok_or_else(|| DecodeError::from(RawValidationError::InvalidImage))?,
            ),
        };
        let metadata = get_process_metadata(&raw)?;
        Ok((mosaic, DngMetadata::default(), metadata))
    }

    fn decode_active(
        &self,
        input: P,
    ) -> Result<(CroppedMosaic<T>, DngMetadata, ProcessMetadata<T>), DecodeError> {
        let input = input.as_ref();
        if !input.is_file() {
            return Err(DecodeError::InvalidInput(
                format!("RAW input is not a file: {}", input.display()).into(),
            ));
        }
        let raw = rawler::decode_file(input).map_err(backend_error)?;
        let mut image = convert_image(&raw)?;
        let process_metadata = get_process_metadata(&raw)?;
        image.crop_in_place(process_metadata.active_area);
        Ok((
            CroppedMosaic::new(image),
            DngMetadata::default(),
            process_metadata,
        ))
    }
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

fn convert_image<T>(raw: &RawImage) -> Result<MonoImage<T>, DecodeError>
where
    T: Primitive,
{
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
    match &raw.data {
        RawImageData::Integer(samples) => {
            if samples.len() != expected {
                return Err(RawValidationError::InvalidImage.into());
            }
            MonoImage::from_raw(width, height, samples.iter().map(|v| num!(*v)).collect())
                .ok_or_else(|| RawValidationError::InvalidImage.into())
        }
        RawImageData::Float(samples) => {
            if samples.len() != expected {
                return Err(RawValidationError::InvalidImage.into());
            }
            if samples.iter().any(|value| !value.is_finite()) {
                return Err(RawValidationError::NonFiniteSample.into());
            }
            MonoImage::from_raw(width, height, samples.iter().map(|v| num!(*v)).collect())
                .ok_or_else(|| RawValidationError::InvalidImage.into())
        }
    }
}

fn get_process_metadata<T>(raw: &RawImage) -> Result<ProcessMetadata<T>, DecodeError>
where
    T: Primitive,
{
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
        let width = u32::try_from(raw.width)
            .map_err(|_| MetadataValidationError::InvalidGeometry(raw.width.to_string().into()))?;
        let height = u32::try_from(raw.height)
            .map_err(|_| MetadataValidationError::InvalidGeometry(raw.height.to_string().into()))?;
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
            black_values.into_iter().map(|v| num!(v)).collect(),
        )
        .ok_or_else(|| MetadataValidationError::InvalidLevels("invalid black-level image".into()))?
    };

    let white = {
        let white_value = raw.whitelevel.0.first().copied().ok_or_else(|| {
            MetadataValidationError::InvalidLevels("white level is missing".into())
        })?;
        let white = num!(white_value);
        if white <= T::zero() {
            return Err(MetadataValidationError::InvalidLevels(
                "white level must be greater than zero".into(),
            )
            .into());
        }
        white
    };

    let bayer_pattern = BayerPattern::from_str(&cfa.name)
        .map_err(|_| DecodeError::Unsupported(format!("unsupported CFA pattern {cfa}").into()))?
        .shifted(active_area.x, active_area.y);
    let wb_coeffs = {
        if let Some(tag) = raw.dng_tags.get(&50728_u16) {
            let v: Vec<f32> = (0..3).map(|i| tag.force_f32(i)).collect();

            Some((num!(v[1]) / num!(v[0]), num!(v[1]) / num!(v[2])))
        } else if raw.neutralwb()[..3]
            .iter()
            .all(|v| *v != 0.0 && !v.is_nan())
        {
            Some((
                num!(raw.neutralwb()[1]) / num!(raw.neutralwb()[0]),
                num!(raw.neutralwb()[1]) / num!(raw.neutralwb()[2]),
            ))
        } else if raw.wb_coeffs[..3].iter().all(|v| *v != 0.0 && !v.is_nan()) {
            // 3. Invert the WB multipliers. G2 is NaN on 3-channel sensors.
            let [r, g, b, _] = raw.wb_coeffs;
            Some((num!(r) / num!(g), num!(b) / num!(g)))
        } else {
            None
        }
    };

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
        wb_coeffs,
    })
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
