use crate::metadata::{DemosaicMetadata, Metadata, ProcessedMetadata};
use crate::{PreprocessError, RawImage, RawValidationError, preprocess};

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

    pub fn preprocess(self) -> Result<(RawImage, ProcessedMetadata), PreprocessError> {
        let Self { image, metadata } = self;
        let area = metadata.preprocess.active_area;
        let full_pattern = metadata.preprocess.cfa.bayer_pattern;
        let image = preprocess::preprocess(image, &metadata.preprocess)?;
        let demosaic = DemosaicMetadata {
            active_area: area,
            bayer_pattern: full_pattern.shift(area.y, area.x),
        };
        Ok((
            image,
            ProcessedMetadata {
                decoder: metadata.decoder,
                demosaic,
                postprocess: metadata.postprocess,
                extra: metadata.extra,
            },
        ))
    }
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
}
