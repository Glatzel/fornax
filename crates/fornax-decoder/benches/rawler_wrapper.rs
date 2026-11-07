use std::path::Path;

use fornax_core::error::{DecodeError, RawValidationError};
use fornax_core::metadata::ProcessMetadata;
use fornax_core::protocol::IDecoder;
use fornax_core::{CroppedMosaic, MonoImage, Primitive, RawMosaic};
use generic_num::num;
use rawler::decoders::{RawImage, RawImageData, RawPhotometricInterpretation};

#[derive(Clone, Debug)]
pub struct RawlerDecoder {
    raw: RawImage,
}

impl IDecoder for RawlerDecoder {
    fn decode(input: &Path) -> Result<Self, DecodeError> {
        if !input.is_file() {
            return Err(DecodeError::InvalidInput(
                format!("RAW input is not a file: {}", input.display()).into(),
            ));
        }
        let raw = rawler::decode_file(input).map_err(|e| match e {
            rawler::RawlerError::Unsupported {
                what,
                model,
                make,
                mode,
            } => DecodeError::Unsupported(
                format!("{what}; make={make}, model={model}, mode={mode}").into(),
            ),
            rawler::RawlerError::DecoderFailed(message) => DecodeError::Backend(message.into()),
        })?;

        Ok(Self { raw })
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
    fn cropped_mosaic<T>(&self) -> Result<CroppedMosaic<T>, DecodeError>
    where
        T: Primitive,
    {
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
        Err(DecodeError::Unsupported("unimplemented"))
    }
}
