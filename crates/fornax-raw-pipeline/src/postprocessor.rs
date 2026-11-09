use std::marker::PhantomData;

use fornax_core::Primitive;
use fornax_core::error::PostProcessError;
use fornax_core::metadata::{Array3, Matrix3x3};
use fornax_core::protocol::{IPostProcessMetadata, IPostProcessor};

use crate::demosaic::{Demosaic, IDemosaic, LazyDemosaic};

/// Color space written by [`PostProcessor`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum OutputColorSpace {
    /// Camera-native RGB; white balance only, no output color-space conversion.
    #[default]
    Raw,

    /// sRGB (IEC 61966-2-1).
    Srgb,

    /// Adobe RGB (1998).
    AdobeRgb,

    /// Wide Gamut RGB (Adobe).
    WideGamutRgb,

    /// ProPhoto RGB (ROMM RGB).
    ProPhotoRgb,

    /// CIE XYZ tristimulus color space (D50 PCS).
    Xyz,

    /// ACES2065-1 (AP0, linear, D60).
    Aces2065_1,

    /// ACEScg (AP1, linear, D60).
    AcesCg,

    /// DCI-P3 (cinema white point).
    DciP3,

    /// ITU-R BT.2020.
    Rec2020,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PostProcessor<T, D>
where
    T: Primitive,
    D: IDemosaic<T>,
{
    pub demosaic: D,
    pub output_color: OutputColorSpace,
    pub _mark: PhantomData<T>,
}

impl<T, D, M> IPostProcessor<T, M> for PostProcessor<T, D>
where
    T: Primitive,
    M: IPostProcessMetadata,
    D: IDemosaic<T>,
{
    fn process(
        &self,
        bayer_image: &fornax_core::MonoImage<T>,
        metadata: &M,
    ) -> Result<fornax_core::RgbImage<T>, PostProcessError> {
        let result = self
            .demosaic
            .demosaic(bayer_image, metadata.bayer_pattern())?;

        // Everything white-point dependent comes from the normalized
        // calibration: gains, and camera -> XYZ D50 on raw values.
        let transform = metadata.color_calibration().resolve();

        if self.output_color == OutputColorSpace::Raw {
            if gains_are_identity(&transform.gains) {
                return Ok(result.collect());
            }
            let gains = cast_vec3::<T>(transform.gains)?;
            return Ok(result.collect_with(|_y, row| {
                for px in row.as_chunks_mut::<3>().0 {
                    px[0] = px[0] * gains[0];
                    px[1] = px[1] * gains[1];
                    px[2] = px[2] * gains[2];
                }
            }));
        }

        // One 3x3 per pixel. `cam_to_xyz_d50` already includes the white
        // balance, so no separate gain pass (that would balance twice).
        let matrix = cast_matrix::<T>(multiply_3x3(
            xyz_to_output(self.output_color),
            transform.cam_to_xyz_d50,
        ))?;
        Ok(result.collect_with(|_y, row| {
            for px in row.as_chunks_mut::<3>().0 {
                let red = matrix[0][0] * px[0] + matrix[0][1] * px[1] + matrix[0][2] * px[2];
                let green = matrix[1][0] * px[0] + matrix[1][1] * px[1] + matrix[1][2] * px[2];
                let blue = matrix[2][0] * px[0] + matrix[2][1] * px[1] + matrix[2][2] * px[2];
                *px = [red, green, blue];
            }
        }))
    }
}

impl<T> Default for PostProcessor<T, Demosaic>
where
    T: Primitive,
{
    fn default() -> Self {
        Self {
            demosaic: Demosaic::Linear,
            output_color: OutputColorSpace::Raw,
            _mark: PhantomData,
        }
    }
}

// ---------------------------------------------------------------------------
// Output color spaces: XYZ D50 -> linear RGB (Bradford-adapted where needed)
// ---------------------------------------------------------------------------

const XYZ_TO_SRGB: Matrix3x3 = [
    [3.133_856_1, -1.616_866_7, -0.490_614_6],
    [-0.978_768_4, 1.916_141_5, 0.033_454],
    [0.071_945_3, -0.228_991_4, 1.405_242_7],
];

const XYZ_TO_ADOBE_RGB: Matrix3x3 = [
    [1.962_427_757_6, -0.610_534_501_4, -0.341_340_496_4],
    [-0.978_768_584_3, 1.916_141_694_2, 0.033_453_969_4],
    [0.028_686_780_3, -0.140_675_150_2, 1.348_765_505_6],
];

const XYZ_TO_WIDE_GAMUT_RGB: Matrix3x3 = [
    [1.462_806_655_9, -0.184_062_333_0, -0.274_360_587_9],
    [-0.521_793_394_4, 1.447_238_130_2, 0.067_722_757_4],
    [0.034_934_235_4, -0.096_893_057_9, 1.288_409_943_4],
];

const XYZ_TO_PROPHOTO_RGB: Matrix3x3 = [
    [1.345_943_366_9, -0.255_607_518_1, -0.051_111_832_4],
    [-0.544_598_822_5, 1.508_167_301_8, 0.020_535_106_0],
    [0.0, 0.0, 1.211_812_750_7],
];

const XYZ_TO_ACES2065_1: Matrix3x3 = [
    [1.015_832_910_2, -0.017_727_653_3, 0.046_341_227_9],
    [-0.507_812_387_5, 1.391_314_012_0, 0.119_156_152_0],
    [0.008_457_236_2, -0.014_031_794_7, 1.218_934_766_1],
];

const XYZ_TO_ACESCG: Matrix3x3 = [
    [1.592_705_210_0, -0.351_775_495_6, -0.222_904_135_8],
    [-0.675_912_837_3, 1.639_260_613_6, 0.015_108_957_0],
    [0.019_949_092_5, -0.022_540_208_0, 1.215_817_663_4],
];

const XYZ_TO_DCI_P3: Matrix3x3 = [
    [2.596_354_355_7, -1.158_211_433_0, -0.418_372_734_0],
    [-0.832_856_687_1, 1.786_266_546_2, 0.020_346_976_7],
    [0.048_660_173_8, -0.099_853_548_3, 1.275_959_374_7],
];

const XYZ_TO_REC2020: Matrix3x3 = [
    [1.647_275_491_8, -0.393_602_322_7, -0.235_980_722_4],
    [-0.682_618_527_6, 1.647_618_099_4, 0.012_816_540_3],
    [0.029_662_711_4, -0.062_916_697_8, 1.253_396_490_9],
];

const IDENTITY: Matrix3x3 = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

const fn xyz_to_output(output_color: OutputColorSpace) -> Matrix3x3 {
    match output_color {
        OutputColorSpace::Raw | OutputColorSpace::Xyz => IDENTITY,
        OutputColorSpace::Srgb => XYZ_TO_SRGB,
        OutputColorSpace::AdobeRgb => XYZ_TO_ADOBE_RGB,
        OutputColorSpace::WideGamutRgb => XYZ_TO_WIDE_GAMUT_RGB,
        OutputColorSpace::ProPhotoRgb => XYZ_TO_PROPHOTO_RGB,
        OutputColorSpace::Aces2065_1 => XYZ_TO_ACES2065_1,
        OutputColorSpace::AcesCg => XYZ_TO_ACESCG,
        OutputColorSpace::DciP3 => XYZ_TO_DCI_P3,
        OutputColorSpace::Rec2020 => XYZ_TO_REC2020,
    }
}

fn multiply_3x3(left: Matrix3x3, right: Matrix3x3) -> Matrix3x3 {
    let mut result = [[0.0; 3]; 3];
    for (row, output) in result.iter_mut().enumerate() {
        for (column, value) in output.iter_mut().enumerate() {
            *value = (0..3)
                .map(|index| left[row][index] * right[index][column])
                .sum();
        }
    }
    result
}

fn gains_are_identity(gains: &Array3) -> bool {
    gains.iter().all(|g| (g - 1.0).abs() < f64::EPSILON)
}

fn cast_value<T>(value: f64) -> Result<T, PostProcessError>
where
    T: Primitive,
{
    num_traits::cast(value).ok_or_else(|| {
        PostProcessError::InvalidColorTransform("color transform exceeds output precision".into())
    })
}

fn cast_vec3<T>(vector: Array3) -> Result<[T; 3], PostProcessError>
where
    T: Primitive,
{
    Ok([
        cast_value(vector[0])?,
        cast_value(vector[1])?,
        cast_value(vector[2])?,
    ])
}

fn cast_matrix<T>(matrix: Matrix3x3) -> Result<[[T; 3]; 3], PostProcessError>
where
    T: Primitive,
{
    Ok([
        cast_vec3(matrix[0])?,
        cast_vec3(matrix[1])?,
        cast_vec3(matrix[2])?,
    ])
}

#[cfg(test)]
mod tests {
    use fornax_core::metadata::{
        ColorCalibration, Levels, LightSource, ProcessMetadata, RawColorMeta, RawProfile,
    };
    use fornax_core::{BayerPattern, MonoImage};
    use image::math::Rect;

    use super::*;

    const D50: [f64; 3] = [0.964_22, 1.0, 0.825_21];

    fn metadata(color: RawColorMeta) -> ProcessMetadata {
        ProcessMetadata {
            active_area: Rect {
                x: 0,
                y: 0,
                width: 8,
                height: 8,
            },
            default_crop: Rect {
                x: 0,
                y: 0,
                width: 8,
                height: 8,
            },
            levels: Levels {
                black: MonoImage::from_raw(1, 1, vec![0.0]).unwrap(),
                black_delta_h: Vec::new(),
                black_delta_v: Vec::new(),
                white: 1.0,
            },
            bayer_pattern: BayerPattern::RGGB,
            color_calibration: ColorCalibration::new(color),
        }
    }

    /// `as_shot_neutral = [1, 1, 1]` means "no white balance": the camera's
    /// (1, 1, 1) is the scene white. Without it the as-shot white falls back
    /// to an assumed D65, and the pipeline (correctly) balances to that.
    fn single_profile(profile: RawProfile) -> RawColorMeta {
        RawColorMeta {
            as_shot_neutral: Some([1.0; 3]),
            profiles: vec![RawProfile {
                illuminant: Some(LightSource::D65),
                ..profile
            }],
            ..Default::default()
        }
    }

    /// FM = CM = identity: camera RGB is XYZ, so the output is just the
    /// XYZ D50 -> output matrix applied to the camera pixel.
    fn identity_profile() -> RawColorMeta {
        single_profile(RawProfile {
            color_matrix: Some(IDENTITY),
            forward_matrix: Some(IDENTITY),
            ..Default::default()
        })
    }

    fn processor(output_color: OutputColorSpace) -> PostProcessor<f32, Demosaic> {
        PostProcessor {
            demosaic: Demosaic::Linear,
            output_color,
            _mark: PhantomData,
        }
    }

    fn gray() -> MonoImage<f32> { MonoImage::from_raw(8, 8, vec![0.1_f32; 8 * 8]).unwrap() }

    #[test]
    fn default_output_color_is_raw() {
        assert_eq!(
            PostProcessor::<f32, Demosaic>::default().output_color,
            OutputColorSpace::Raw
        );
    }

    #[test]
    fn d50_pcs_maps_to_neutral_in_each_rgb_output_space() {
        let output_spaces = [
            OutputColorSpace::Srgb,
            OutputColorSpace::AdobeRgb,
            OutputColorSpace::WideGamutRgb,
            OutputColorSpace::ProPhotoRgb,
            OutputColorSpace::Aces2065_1,
            OutputColorSpace::AcesCg,
            OutputColorSpace::DciP3,
            OutputColorSpace::Rec2020,
        ];

        for output_color in output_spaces {
            let matrix = xyz_to_output(output_color);
            for (row, expected) in matrix.into_iter().zip([1.0; 3]) {
                let actual = f64::mul_add(
                    row[0],
                    D50[0],
                    f64::mul_add(row[1], D50[1], row[2] * D50[2]),
                );
                assert!(
                    (actual - expected).abs() < 1.0e-6,
                    "{output_color:?}: {actual}"
                );
            }
        }
    }

    #[test]
    fn raw_output_preserves_demosaicked_camera_rgb() -> mischief::Result<()> {
        let metadata = metadata(single_profile(RawProfile {
            color_matrix: Some([[2.0, 0.0, 0.0], [0.0, 3.0, 0.0], [0.0, 0.0, 4.0]]),
            ..Default::default()
        }));

        let output = processor(OutputColorSpace::Raw).process(&gray(), &metadata)?;
        let pixel = output.get_pixel(4, 4);

        assert!((pixel[0] - 0.1).abs() < 1.0e-6, "{pixel:?}");
        assert!((pixel[1] - 0.1).abs() < 1.0e-6, "{pixel:?}");
        assert!((pixel[2] - 0.1).abs() < 1.0e-6, "{pixel:?}");
        Ok(())
    }

    #[test]
    fn each_output_color_space_uses_its_xyz_transform() -> mischief::Result<()> {
        let metadata = metadata(identity_profile());
        let cases = [
            (OutputColorSpace::Srgb, [1.0263748, 0.9708271, 1.2481966]),
            (
                OutputColorSpace::AdobeRgb,
                [1.0105528, 0.9708271, 1.2367771],
            ),
            (
                OutputColorSpace::WideGamutRgb,
                [1.0043837, 0.9931675, 1.2264511],
            ),
            (
                OutputColorSpace::ProPhotoRgb,
                [1.039224, 0.9841036, 1.2118128],
            ),
            (OutputColorSpace::Xyz, [1.0, 1.0, 1.0]),
            (
                OutputColorSpace::Aces2065_1,
                [1.0444465, 1.0026578, 1.2133602],
            ),
            (OutputColorSpace::AcesCg, [1.0180256, 0.9784567, 1.2132266]),
            (OutputColorSpace::DciP3, [1.0197702, 0.9737568, 1.224766]),
            (OutputColorSpace::Rec2020, [1.0176925, 0.9778161, 1.2201425]),
        ];

        for (output_color, expected) in cases {
            let output = processor(output_color).process(&gray(), &metadata)?;
            let pixel = output.get_pixel(4, 4);

            for (actual, expected) in pixel.0.into_iter().zip(expected) {
                assert!(
                    f32::mul_add(expected, -0.1, actual).abs() < 1.0e-6,
                    "{output_color:?}: {pixel:?}"
                );
            }
        }
        Ok(())
    }

    /// ColorMatrix path: whatever AB/CC/CM are, the neutral must land on the
    /// D50 white (Y = 1), so a gray camera pixel becomes 0.1 * D50.
    #[test]
    fn color_matrix_path_maps_neutral_to_d50() -> mischief::Result<()> {
        let metadata = metadata(single_profile(RawProfile {
            color_matrix: Some(IDENTITY),
            camera_calibration: Some([[2.0, 0.0, 0.0], [0.0, 4.0, 0.0], [0.0, 0.0, 5.0]]),
            ..Default::default()
        }));

        let output = processor(OutputColorSpace::Xyz).process(&gray(), &metadata)?;
        let pixel = output.get_pixel(4, 4);

        for (actual, expected) in pixel.0.into_iter().zip(D50) {
            assert!(
                0.1_f64.mul_add(-expected, f64::from(actual)).abs() < 1.0e-5,
                "{pixel:?}"
            );
        }
        Ok(())
    }

    #[test]
    fn forward_matrix_is_used_when_every_profile_has_one() -> mischief::Result<()> {
        let metadata = metadata(single_profile(RawProfile {
            forward_matrix: Some([[2.0, 0.0, 0.0], [0.0, 2.0, 0.0], [0.0, 0.0, 2.0]]),
            color_matrix: Some([[0.5, 0.0, 0.0], [0.0, 0.5, 0.0], [0.0, 0.0, 0.5]]),
            ..Default::default()
        }));

        let output = processor(OutputColorSpace::Xyz).process(&gray(), &metadata)?;
        let pixel = output.get_pixel(4, 4);

        assert!((pixel[0] - 0.2).abs() < 1.0e-6, "{pixel:?}");
        assert!((pixel[1] - 0.2).abs() < 1.0e-6, "{pixel:?}");
        assert!((pixel[2] - 0.2).abs() < 1.0e-6, "{pixel:?}");
        Ok(())
    }

    /// Maker gains (R, B; G = 1) become a neutral, and the raw output
    /// multiplies every channel by 1 / neutral.
    #[test]
    fn white_balance_multiplies_each_channel_by_its_gain() -> mischief::Result<()> {
        let metadata = metadata(RawColorMeta {
            maker_gains: Some((2.0, 1.5)),
            ..Default::default()
        });

        let output = processor(OutputColorSpace::Raw).process(&gray(), &metadata)?;
        let pixel = output.get_pixel(4, 4);

        assert!((pixel[0] - 0.2).abs() < 1.0e-6, "{pixel:?}");
        assert!((pixel[1] - 0.1).abs() < 1.0e-6, "{pixel:?}");
        assert!((pixel[2] - 0.15).abs() < 1.0e-6, "{pixel:?}");
        Ok(())
    }

    /// With color output the white balance lives inside the matrix, applied
    /// once: FM = I and neutral (0.5, 1, 0.5) give a diagonal (2, 1, 2).
    #[test]
    fn white_balance_is_applied_once_with_color_output() -> mischief::Result<()> {
        let mut color = identity_profile();
        color.as_shot_neutral = Some([0.5, 1.0, 0.5]);
        let metadata = metadata(color);

        let output = processor(OutputColorSpace::Xyz).process(&gray(), &metadata)?;
        let pixel = output.get_pixel(4, 4);

        assert!((pixel[0] - 0.2).abs() < 1.0e-6, "{pixel:?}");
        assert!((pixel[1] - 0.1).abs() < 1.0e-6, "{pixel:?}");
        assert!((pixel[2] - 0.2).abs() < 1.0e-6, "{pixel:?}");
        Ok(())
    }
}
