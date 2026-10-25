use crate::metadata::{LevelsMetadata, PreprocessMetadata, WhiteLevels};
use crate::{PreprocessError, RawImage};

pub fn preprocess(
    image: RawImage,
    metadata: &PreprocessMetadata,
) -> Result<RawImage, PreprocessError> {
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
    use image::ImageBuffer;
    use image::math::Rect;

    use super::*;
    use crate::BayerPattern;
    use crate::metadata::{BlackLevels, CfaMetadata};

    fn metadata() -> PreprocessMetadata {
        PreprocessMetadata {
            active_area: Rect {
                x: 0,
                y: 0,
                width: 2,
                height: 2,
            },
            cfa: CfaMetadata {
                bayer_pattern: BayerPattern::Rggb,
            },
            levels: LevelsMetadata {
                black: BlackLevels {
                    common: 10,
                    per_slot: [0, 1, 2, 3],
                },
                white: WhiteLevels::Uniform(110),
            },
        }
    }

    #[test]
    fn normalizes_each_bayer_slot_without_changing_dimensions() {
        let image =
            RawImage::try_new(ImageBuffer::from_raw(2, 2, vec![60.0, 60.5, 61.5, 61.0]).unwrap())
                .unwrap();
        let output = preprocess(image, &metadata()).unwrap();
        assert_eq!(output.full_image().dimensions(), (2, 2));
        for sample in output.full_image().as_raw() {
            assert!((sample - 0.5).abs() < 1e-6);
        }
    }

    #[test]
    fn rejects_invalid_levels_and_crop() {
        let mut levels = metadata();
        levels.levels.white = WhiteLevels::Uniform(10);
        let image = RawImage::try_new(ImageBuffer::from_raw(2, 2, vec![0.0; 4]).unwrap()).unwrap();
        assert_eq!(
            preprocess(image, &levels),
            Err(PreprocessError::InvalidLevelRange)
        );

        let mut crop = metadata();
        crop.active_area.width = 3;
        let image = RawImage::try_new(ImageBuffer::from_raw(2, 2, vec![0.0; 4]).unwrap()).unwrap();
        assert_eq!(
            preprocess(image, &crop),
            Err(PreprocessError::InvalidActiveArea)
        );
    }
}
