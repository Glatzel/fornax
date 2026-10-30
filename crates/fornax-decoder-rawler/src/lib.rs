use std::path::Path;

use fornax_core::error::{DecodeError, MetadataValidationError, RawValidationError};
use fornax_core::metadata::{Levels, Metadata, WhiteBalance};
use fornax_core::protocol::IDecoder;
use fornax_core::{BayerPattern, DecodedRawActive, DecodedRawFull, MonoImage, Primitive};
use generic_num::num;
use image::math::Rect;
use rawler::decoders::{RawImage, RawImageData, RawPhotometricInterpretation};

#[derive(Clone, Copy, Debug, Default)]
pub struct RawlerDecoder;

impl<T> IDecoder<Path, T> for RawlerDecoder
where
    T: Primitive,
{
    fn decode_full(&self, input: &Path) -> Result<(DecodedRawFull<T>, Metadata<T>), DecodeError> {
        decode(input)
    }

    fn decode_active(
        &self,
        input: &Path,
    ) -> Result<(DecodedRawActive<T>, Metadata<T>), DecodeError> {
        let (full, metadata) = decode(input)?;
        Ok((full.active_area(metadata.active_area), metadata))
    }
}

fn decode<T>(input: &Path) -> Result<(DecodedRawFull<T>, Metadata<T>), DecodeError>
where
    T: Primitive,
{
    if !input.is_file() {
        return Err(DecodeError::InvalidInput(
            format!("RAW input is not a file: {}", input.display()).into(),
        ));
    }

    let raw = rawler::decode_file(input).map_err(backend_error)?;
    let image = convert_image(&raw)?;
    let metadata = convert_metadata(&raw)?;
    Ok((DecodedRawFull::new(image), metadata))
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

fn convert_metadata<T>(raw: &RawImage) -> Result<Metadata<T>, DecodeError>
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

    let bayer_pattern = match cfa.name.to_ascii_uppercase().as_str() {
        "RGGB" => BayerPattern::RGGB,
        "BGGR" => BayerPattern::BGGR,
        "GRBG" => BayerPattern::GRBG,
        "GBRG" => BayerPattern::GBRG,
        _ => {
            return Err(DecodeError::Unsupported(
                format!("unsupported CFA pattern {cfa}").into(),
            ));
        }
    }
    .shifted(active_area.x, active_area.y);
    let as_shot_neutral = 'neutral: {
        let finite_pos = |v: f32| v.is_finite() && v > 0.0;

        // 1. The file's own AsShotNeutral tag (50728): [R, G, B], already
        //    neutral.
        if let Some(tag) = raw.dng_tags.get(&50728_u16) {
            let v: Vec<f32> = (0..3).map(|i| tag.force_f32(i)).collect(); // adapt to your Tag API
            if v.iter().copied().all(finite_pos) {
                break 'neutral Some(WhiteBalance::AsShotNeutral(
                    num!(v[0]),
                    num!(v[1]),
                    num!(v[2]),
                ));
            }
        }

        // 2. rawler's own neutral, if sane.
        let n = raw.neutralwb();
        if n[..3].iter().copied().all(finite_pos) {
            break 'neutral Some(WhiteBalance::AsShotNeutral(
                num!(n[0]),
                num!(n[1]),
                num!(n[2]),
            ));
        }

        // 3. Invert the WB multipliers. G2 is NaN on 3-channel sensors.
        let [r, g, b, g2] = raw.wb_coeffs;
        if ![r, g, b].into_iter().all(finite_pos) {
            break 'neutral None;
        }
        let g = if finite_pos(g2) {
            f32::midpoint(g, g2)
        } else {
            g
        };

        Some(WhiteBalance::AsShotNeutral(num!(r), num!(g), num!(b)))
    };
    Ok(Metadata {
        active_area,
        default_crop,
        levels: Levels {
            black,
            black_delta_h: Vec::new(),
            black_delta_v: Vec::new(),
            white,
        },
        bayer_pattern,
        white_balance: as_shot_neutral,
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

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    fn decode_with<T: Primitive>() -> Result<(), Box<dyn std::error::Error>> {
        let decoder = RawlerDecoder;
        let path = fornax_devtool::raw_file();
        let (full, full_metadata) =
            <RawlerDecoder as IDecoder<Path, T>>::decode_full(&decoder, &path)?;
        let (active, active_metadata) =
            <RawlerDecoder as IDecoder<Path, T>>::decode_active(&decoder, &path)?;

        assert_eq!(full.image().dimensions(), (5_360, 3_516));
        assert_eq!(
            active.image().dimensions(),
            (5_202, 3_464),
            "active={:?}, crop={:?}",
            full_metadata.active_area,
            full_metadata.default_crop
        );
        assert_eq!(
            full_metadata.active_area,
            Rect {
                x: 158,
                y: 52,
                width: 5_202,
                height: 3_464,
            }
        );
        assert_eq!(active_metadata.active_area, full_metadata.active_area);
        assert_eq!(
            full_metadata.default_crop,
            Rect {
                x: 10,
                y: 4,
                width: 5_184,
                height: 3_456,
            }
        );
        assert_eq!(active_metadata.levels.black.dimensions(), (2, 2));
        assert!(active_metadata.levels.white > T::zero());
        assert_eq!(active_metadata.bayer_pattern, BayerPattern::GBRG);
        Ok(())
    }

    #[test]
    fn decodes_cr2_as_f32() -> Result<(), Box<dyn std::error::Error>> { decode_with::<f32>() }

    #[test]
    fn decodes_cr2_as_f64() -> Result<(), Box<dyn std::error::Error>> { decode_with::<f64>() }

    #[test]
    fn missing_file_returns_invalid_input() {
        let result = <RawlerDecoder as IDecoder<Path, f32>>::decode_full(
            &RawlerDecoder,
            Path::new("this-file-does-not-exist.cr2"),
        );
        let error = result.expect_err("missing input must fail");
        assert!(matches!(error, DecodeError::InvalidInput(_)));
        assert!(error.to_string().contains("this-file-does-not-exist.cr2"));
    }
}
