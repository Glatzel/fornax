use crate::metadata::{
    DemosaicMetadata, LevelsMetadata, Metadata, PreprocessMetadata, ProcessedMetadata, WhiteLevels,
};
use crate::{PreprocessError, RawImage, RawValidationError};

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

#[cfg(test)]
mod tests {
    use image::math::Rect;
    use image::{GenericImageView, ImageBuffer};

    use super::*;
    use crate::BayerPattern;
    use crate::metadata::{
        BlackLevels, CfaMetadata, DecoderMetadata, ExtraMetadata, LevelsMetadata,
        PostprocessMetadata, PreprocessMetadata, WhiteLevels,
    };

    fn metadata() -> Metadata {
        Metadata {
            decoder: DecoderMetadata::default(),
            preprocess: PreprocessMetadata {
                active_area: Rect {
                    x: 1,
                    y: 1,
                    width: 2,
                    height: 2,
                },
                cfa: CfaMetadata {
                    bayer_pattern: BayerPattern::Rggb,
                },
                levels: LevelsMetadata {
                    black: BlackLevels::default(),
                    white: WhiteLevels::Uniform(100),
                },
            },
            postprocess: PostprocessMetadata::default(),
            extra: ExtraMetadata::new(),
        }
    }

    fn decoded(samples: Vec<f32>) -> DecodedRaw {
        let image = RawImage::try_new(ImageBuffer::from_raw(3, 3, samples).unwrap()).unwrap();
        DecodedRaw::try_new(image, metadata()).unwrap()
    }

    #[test]
    fn retains_raw_codes_before_preprocessing() {
        let decoded = decoded(vec![1000.0; 9]);
        assert_eq!(decoded.image().full_image().as_raw()[0], 1000.0);
    }

    #[test]
    fn preprocess_preserves_full_dimensions_and_returns_remaining_metadata() {
        let (image, metadata) = decoded(vec![50.0; 9]).preprocess().unwrap();
        assert_eq!(image.full_image().dimensions(), (3, 3));
        assert_eq!(metadata.demosaic.active_area.width, 2);
        assert_eq!(metadata.demosaic.bayer_pattern, BayerPattern::Bggr);
    }

    #[test]
    fn image_crop_is_a_metadata_free_view() {
        let decoded = decoded(vec![25.0; 9]);
        let (image, metadata) = decoded.preprocess().unwrap();
        let view = image.cropped(metadata.demosaic.active_area).unwrap();
        assert_eq!(view.dimensions(), (2, 2));
        assert_eq!(view.get_pixel(0, 0)[0], 0.25);
    }

    #[test]
    fn preprocess_uses_each_bayer_slot_level() {
        let mut metadata = metadata();
        metadata.preprocess.levels.black.per_slot = [0, 1, 2, 3];
        metadata.preprocess.levels.white = WhiteLevels::PerSlot([100, 101, 102, 103]);
        let row0 = [50.0, 51.0, 50.0, 51.0];
        let row1 = [53.0, 52.0, 53.0, 52.0];
        let samples = [row0, row1, row0].concat();
        let image = RawImage::try_new(ImageBuffer::from_raw(4, 3, samples).unwrap()).unwrap();
        let (image, _) = DecodedRaw::try_new(image, metadata)
            .unwrap()
            .preprocess()
            .unwrap();
        for sample in image.full_image().as_raw() {
            assert!((*sample - 0.5).abs() < 1e-6);
        }
    }

    #[test]
    fn preprocess_rejects_invalid_levels_and_crop() {
        let mut invalid_levels = metadata();
        invalid_levels.preprocess.levels.black.common = 10;
        invalid_levels.preprocess.levels.white = WhiteLevels::Uniform(10);
        let image = RawImage::try_new(ImageBuffer::from_raw(3, 3, vec![0.0; 9]).unwrap()).unwrap();
        assert_eq!(
            DecodedRaw::try_new(image, invalid_levels)
                .unwrap()
                .preprocess(),
            Err(PreprocessError::InvalidLevelRange)
        );

        let mut invalid_crop = metadata();
        invalid_crop.preprocess.active_area.width = 4;
        let image = RawImage::try_new(ImageBuffer::from_raw(3, 3, vec![0.0; 9]).unwrap()).unwrap();
        assert_eq!(
            DecodedRaw::try_new(image, invalid_crop),
            Err(RawValidationError::InvalidImage)
        );
    }
}
