use std::fs;
use std::path::Path;

use fornax_core::error::DecodeError;
use fornax_core::metadata::dng::{DngMetadata, DngTag};
use fornax_core::metadata::process::{Levels, ProcessMetadata};
use fornax_core::protocol::IDecoder;
use fornax_core::{BayerChannel, BayerPattern, DecodedRaw, MonoImage, MosaicData, Primitive};
use image::math::Rect;
use num_traits::{NumCast, ToPrimitive};
use tiff::decoder::ifd::Value;

use super::{Decoder, dng_decoder};

impl<T> IDecoder<&Path, T> for Decoder
where
    T: Primitive,
{
    fn decode(&self, input: &Path) -> Result<(MosaicData, DngMetadata), DecodeError> {
        let (pixels, dng_metadata) = decode_file(input)?;
        let metadata = metadata_from_dng(&pixels, &dng_metadata)?;
        Ok((pixels, metadata))
    }

    fn decode_active(&self, input: &Path) -> Result<(DecodedRaw<T>, DngMetadata), DecodeError> {
        let (pixels, mut metadata) = <Self as IDecoder<&Path, T>>::decode(self, input)?;
        let active_area = metadata.active_area().map_err(|e|)?;
        metadata.bayer_pattern = metadata.bayer_pattern.shifted(active_area.x, active_area.y);
        let mut image = mosaic_to_mono::<T>(&pixels)?;
        image.crop_in_place(active_area);
        Ok((DecodedRaw::new(image), metadata))
    }
}

fn decode_file(input: &Path) -> Result<(MosaicData, DngMetadata), DecodeError> {
    let source = fs::read(input).map_err(|error| {
        DecodeError::InvalidInput(format!("failed to read {}: {error}", input.display()).into())
    })?;
    dng_decoder::decode(&source)
}

fn mosaic_to_mono<T>(decoded: &MosaicData) -> Result<MonoImage<T>, DecodeError>
where
    T: Primitive,
{
    let (width, height) = decoded.dimensions();
    let samples: Vec<Option<T>> = match decoded {
        MosaicData::U8(image) => image.as_raw().iter().map(|&value| cast(value)).collect(),
        MosaicData::U16(image) => image.as_raw().iter().map(|&value| cast(value)).collect(),
        MosaicData::U32(image) => image.as_raw().iter().map(|&value| cast(value)).collect(),
        MosaicData::F32(image) => image.as_raw().iter().map(|&value| cast(value)).collect(),
    };
    let samples = samples
        .into_iter()
        .collect::<Option<Vec<T>>>()
        .ok_or_else(|| DecodeError::Backend("sample does not fit target type".into()))?;
    MonoImage::from_raw(width, height, samples)
        .ok_or_else(|| DecodeError::Backend("decoded mosaic dimensions are invalid".into()))
}

fn cast<T, U>(value: U) -> Option<T>
where
    T: Primitive,
    U: NumCast,
{
    num_traits::cast(value)
}

