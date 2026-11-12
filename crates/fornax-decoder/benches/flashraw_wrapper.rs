use std::path::Path;

use flashraw::{
    Backend, CfaPattern, CorruptionPolicy, DecodeOptions, FlashRaw, OutputFormat, RawPixelData,
    Rect, SampleFormat,
};
use fornax_raw_core::error::{DecodeError, RawValidationError};
use fornax_raw_core::metadata::ProcessMetadata;
use fornax_raw_core::protocol::IDecoder;
use fornax_raw_core::{CroppedMosaic, MonoImage, Primitive, RawMosaic};
use generic_num::num;
use image::math::Rect as ImageRect;

#[derive(Clone, Debug)]
pub struct FlashRawDecoder {
    raw: RawMosaic,
    active_area: ImageRect,
}

impl FlashRawDecoder {
    pub fn from_parts(
        width: u32,
        height: u32,
        cpp: u32,
        cfa: Option<CfaPattern>,
        data: RawPixelData,
        active_area: Rect,
    ) -> Result<Self, DecodeError> {
        if cpp != 1 {
            return Err(DecodeError::Unsupported(
                format!("FlashRaw expected one sample per pixel, got cpp={cpp}").into(),
            ));
        }
        if !cfa.is_some_and(|cfa| cfa.is_bayer()) {
            return Err(DecodeError::Unsupported(
                format!("FlashRaw output is not a 2x2 RGB Bayer frame: {cfa:?}").into(),
            ));
        }
        let expected = usize::try_from(u64::from(width) * u64::from(height))
            .map_err(|_| RawValidationError::InvalidImage)?;
        let raw = match data {
            RawPixelData::U16(samples) => {
                validate_sample_count(samples.len(), expected)?;
                RawMosaic::U16(
                    MonoImage::from_raw(width, height, samples)
                        .ok_or(RawValidationError::InvalidImage)?,
                )
            }
            RawPixelData::F32(samples) => {
                validate_sample_count(samples.len(), expected)?;
                if samples.iter().any(|value| !value.is_finite()) {
                    return Err(RawValidationError::NonFiniteSample.into());
                }
                RawMosaic::F32(
                    MonoImage::from_raw(width, height, samples)
                        .ok_or(RawValidationError::InvalidImage)?,
                )
            }
            other => {
                return Err(DecodeError::Unsupported(
                    format!("FlashRaw returned unsupported raw sample storage {other:?}").into(),
                ));
            }
        };
        validate_active_area(active_area, width, height)?;
        Ok(Self {
            raw,
            active_area: ImageRect {
                x: active_area.x,
                y: active_area.y,
                width: active_area.width,
                height: active_area.height,
            },
        })
    }
}

impl IDecoder for FlashRawDecoder {
    fn decode(input: &Path) -> Result<Self, DecodeError> {
        let raw = FlashRaw::open_path(input)
            .map_err(|error| DecodeError::Backend(format!("FlashRaw: {error}").into()))?;
        let output = match raw.frame_info().sample_format {
            SampleFormat::U16 => OutputFormat::RawU16,
            SampleFormat::F32 => OutputFormat::RawF32,
            other => {
                return Err(DecodeError::Unsupported(
                    format!("FlashRaw sample format {other:?} is unsupported").into(),
                ));
            }
        };
        let frame = raw
            .decode(DecodeOptions {
                backend: Backend::Cpu,
                output,
                threads: Some(1),
                crop: false,
                apply_opcodes: false,
                corruption_policy: CorruptionPolicy::Strict,
                ..DecodeOptions::default()
            })
            .map_err(|error| DecodeError::Backend(format!("FlashRaw: {error}").into()))?;
        Self::from_parts(
            frame.width,
            frame.height,
            frame.cpp,
            frame.cfa,
            frame.data.clone(),
            frame.active_area,
        )
    }

    fn raw_mosaic(&self) -> Result<RawMosaic, DecodeError> { Ok(self.raw.clone()) }

    fn cropped_mosaic<T>(&self) -> Result<CroppedMosaic<T>, DecodeError>
    where
        T: Primitive,
    {
        let (width, height) = self.raw.dimensions();
        let samples = match &self.raw {
            RawMosaic::U8(samples) => samples.as_raw().iter().map(|v| num!(*v)).collect(),
            RawMosaic::U16(samples) => samples.as_raw().iter().map(|v| num!(*v)).collect(),
            RawMosaic::U32(samples) => samples.as_raw().iter().map(|v| num!(*v)).collect(),
            RawMosaic::F32(samples) => samples.as_raw().iter().map(|v| num!(*v)).collect(),
        };
        let mut cropped = MonoImage::from_raw(width, height, samples)
            .ok_or(RawValidationError::InvalidImage)?;
        cropped.crop_in_place(self.active_area);
        cropped.shrink_to_fit();
        Ok(CroppedMosaic::new(cropped))
    }

    fn process_metadata(&self) -> Result<ProcessMetadata, DecodeError> {
        Err(DecodeError::Unsupported(
            "FlashRaw process metadata is not implemented".into(),
        ))
    }
}

fn validate_sample_count(actual: usize, expected: usize) -> Result<(), DecodeError> {
    if actual == expected {
        Ok(())
    } else {
        Err(RawValidationError::InvalidImage.into())
    }
}

fn validate_active_area(active_area: Rect, width: u32, height: u32) -> Result<(), DecodeError> {
    if active_area.width == 0
        || active_area.height == 0
        || u64::from(active_area.x) + u64::from(active_area.width) > u64::from(width)
        || u64::from(active_area.y) + u64::from(active_area.height) > u64::from(height)
    {
        return Err(DecodeError::InvalidInput(
            format!(
                "FlashRaw active area {active_area:?} is outside the {width}x{height} frame"
            )
            .into(),
        ));
    }
    Ok(())
}
