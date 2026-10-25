use crate::RawImage;
use crate::error::{PreprocessError, RawValidationError};
use crate::metadata::{LevelsMetadata, Metadata, PreprocessMetadata, WhiteLevels};

#[derive(Clone, Debug, PartialEq)]
pub struct DecodedRaw {
    image: RawImage,
    metadata: Metadata,
}

impl DecodedRaw {
    pub fn try_new(image: RawImage, metadata: Metadata) -> Result<Self, RawValidationError> {
        image
            .cropped(metadata.preprocess.active_area)
            .map_err(|_| RawValidationError::InvalidImage)?;
        Ok(Self { image, metadata })
    }

    pub const fn image(&self) -> &RawImage { &self.image }

    pub const fn metadata(&self) -> &Metadata { &self.metadata }

    pub fn into_parts(self) -> (RawImage, Metadata) { (self.image, self.metadata) }

    pub fn preprocess(self) -> Result<(RawImage, Metadata), PreprocessError> {
        let Self { image, metadata } = self;
        let image = preprocess(image, &metadata.preprocess)?;
        Ok((image, metadata))
    }
}

fn preprocess(image: RawImage, metadata: &PreprocessMetadata) -> Result<RawImage, PreprocessError> {
    image
        .cropped(metadata.active_area)
        .map_err(|_| PreprocessError::InvalidActiveArea)?;
    let ranges = level_ranges(&metadata.levels)?;
    let codes = metadata.cfa.bayer_pattern.codes();
    let width = image.full_image().width() as usize;
    let mut plane = image.into_full_image();
    for (index, sample) in plane.as_mut().iter_mut().enumerate() {
        let row = index / width;
        let column = index % width;
        let code = codes[(row & 1) * 2 + (column & 1)] as usize;
        let (black, range) = ranges[code];
        *sample = ((f64::from(*sample) - black) / range).clamp(0.0, 1.0) as f32;
    }
    RawImage::try_new(plane).map_err(PreprocessError::InvalidImage)
}

fn level_ranges(levels: &LevelsMetadata) -> Result<[(f64, f64); 4], PreprocessError> {
    let mut ranges = [(0.0, 0.0); 4];
    for (code, range) in ranges.iter_mut().enumerate() {
        let black = f64::from(levels.black.common) + f64::from(levels.black.per_slot[code]);
        let white = match levels.white {
            WhiteLevels::Uniform(white) => f64::from(white),
            WhiteLevels::PerSlot(white) => f64::from(white[code]),
        };
        if !white.is_finite() || white <= black {
            return Err(PreprocessError::InvalidLevelRange);
        }
        *range = (black, white - black);
    }
    Ok(ranges)
}
