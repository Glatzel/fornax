//! Bilinear Bayer demosaicing.
//!
//! # Examples
//!
//! ```
//! use fornax::demosaic::DemosaicAlgorithm;
//!
//! assert_eq!(DemosaicAlgorithm::default(), DemosaicAlgorithm::Linear);
//! ```

use crate::RawImage;
use crate::demosaic::RgbImageF32;
use crate::error::DemosaicError;
use crate::geometry::PixelCoord;
use crate::metadata::{BayerLayout, BayerPattern, CfaColor};

#[derive(Clone, Copy)]
enum BayerPhase {
    Red,
    GreenHorizontalRed,
    GreenVerticalRed,
    Blue,
}

#[inline(always)]
const fn idx(width: u32, x: u32, y: u32) -> usize { y as usize * width as usize + x as usize }

fn phase_at(layout: BayerLayout, x: u32, y: u32) -> BayerPhase {
    let slot = layout
        .slot_at(PixelCoord::new(y, x))
        .expect("every stored coordinate is inside the validated ActiveArea");
    match layout.pattern().color_at(slot) {
        CfaColor::Red => BayerPhase::Red,
        CfaColor::Blue => BayerPhase::Blue,
        CfaColor::Green => {
            if primary_green_slot(layout.pattern()) == slot.index() {
                BayerPhase::GreenHorizontalRed
            } else {
                BayerPhase::GreenVerticalRed
            }
        }
        CfaColor::Other(_) => {
            unreachable!("BayerLayout cannot contain non-RGB colors")
        }
    }
}

const fn primary_green_slot(pattern: BayerPattern) -> u8 {
    match pattern {
        BayerPattern::Rggb => 1,
        BayerPattern::Bggr => 2,
        BayerPattern::Grbg => 0,
        BayerPattern::Gbrg => 3,
    }
}

/// Reconstructs linear RGB using bilinear interpolation.
///
/// Known samples retain their input value. Missing channels average all
/// in-bounds neighbours of the relevant color, including at image borders.
///
/// # Errors
///
/// Returns [`DemosaicError::InvalidInput`] if the source dimensions are
/// invalid. `RawImage` already guarantees finite samples in `[0,1]`.
///
/// # Examples
///
/// ```
/// # use std::num::{NonZeroU32, NonZeroU64};
/// # use fornax::geometry::{ActiveArea, PixelSize, StoredGeometry};
/// # use fornax::metadata::{
/// #     AuthorityClass, BlackLevels, CommonMetadata, Confidence, DecoderProvenance,
/// #     ExtensibleMetadata, Provenance, ProviderId, Rational, SourceKind, Sourced,
/// #     ValidatedMetadata, WhiteLevels,
/// # };
/// # use fornax::metadata::{BayerLayout, BayerPattern};
/// # use fornax::{
/// #     DecodedRaw, RawImage, SampleFormat, SampleSpec, StoredPlane, into_preprocessed,
/// # };
/// # fn input() -> RawImage {
/// #     let dimensions = PixelSize::new(1, 1).unwrap();
/// #     let area = ActiveArea::try_new(dimensions, 0, 0, 1, 1).unwrap();
/// #     let provider = ProviderId::try_new("example").unwrap();
/// #     let provenance = Provenance::new(
/// #         provider.clone(),
/// #         SourceKind::Derived { operation: "example".into() },
/// #         AuthorityClass::DerivedValue,
/// #         Confidence::Exact,
/// #     );
/// #     let zero = Rational::new(0, NonZeroU32::new(1).unwrap());
/// #     let one = Rational::new(1, NonZeroU32::new(1).unwrap());
/// #     let metadata = ValidatedMetadata::try_new(
/// #         CommonMetadata {
/// #             sample_spec: Some(Sourced::new(
/// #                 SampleSpec::try_new(32, SampleFormat::Float).unwrap(),
/// #                 provenance.clone(),
/// #             )),
/// #             stored_geometry: Some(Sourced::new(
/// #                 StoredGeometry::new(dimensions, NonZeroU64::new(4).unwrap()),
/// #                 provenance.clone(),
/// #             )),
/// #             active_area: Some(Sourced::new(area, provenance.clone())),
/// #             cfa: Some(Sourced::new(
/// #                 BayerLayout::new(BayerPattern::Rggb, area),
/// #                 provenance.clone(),
/// #             )),
/// #             black: Some(Sourced::new(
/// #                 BlackLevels::try_new(zero, [zero; 4], None).unwrap(),
/// #                 provenance.clone(),
/// #             )),
/// #             white: Some(Sourced::new(
/// #                 WhiteLevels::try_new(vec![one], None, None).unwrap(),
/// #                 provenance,
/// #             )),
/// #             ..CommonMetadata::default()
/// #         },
/// #         ExtensibleMetadata::default(),
/// #         DecoderProvenance::new(provider, None),
/// #     )
/// #     .unwrap();
/// #     let plane = image::ImageBuffer::from_raw(1, 1, vec![0.25]).unwrap();
/// #     let decoded = DecodedRaw::try_new(metadata, StoredPlane::F32(plane.clone())).unwrap();
/// #     into_preprocessed(decoded, plane).unwrap()
/// # }
/// let output = fornax::demosaic::linear::demosaic(&input()).unwrap();
/// assert_eq!(output.get_pixel(0, 0).0, [0.25, 0.0, 0.0]);
/// ```
pub fn demosaic(input: &RawImage) -> Result<RgbImageF32, DemosaicError> {
    let mosaic = input.plane();
    let (width, height) = mosaic.dimensions();
    if width == 0 || height == 0 {
        return Err(DemosaicError::InvalidInput(
            "RAW image dimensions must be non-zero".into(),
        ));
    }
    let layout = input.metadata().cfa();
    let raw = mosaic.as_raw();
    let row_stride = usize::try_from(width)
        .ok()
        .and_then(|width| width.checked_mul(3))
        .ok_or_else(|| DemosaicError::InvalidInput("RGB row size overflows usize".into()))?;
    let mut output = RgbImageF32::new(width, height);

    #[cfg(feature = "parallel")]
    {
        use rayon::prelude::*;

        output
            .as_mut()
            .par_chunks_mut(row_stride)
            .enumerate()
            .for_each(|(y, row)| write_row(raw, width, height, layout, y as u32, row));
    }

    #[cfg(not(feature = "parallel"))]
    output
        .as_mut()
        .chunks_mut(row_stride)
        .enumerate()
        .for_each(|(y, row)| write_row(raw, width, height, layout, y as u32, row));

    Ok(output)
}

fn write_row(
    raw: &[f32],
    width: u32,
    height: u32,
    layout: BayerLayout,
    y: u32,
    output: &mut [f32],
) {
    let border_row = y == 0 || y + 1 == height;
    for x in 0..width {
        let phase = phase_at(layout, x, y);
        let centre = raw[idx(width, x, y)];
        let rgb = if border_row || x == 0 || x + 1 == width {
            checked_pixel(raw, width, height, x, y, phase, centre)
        } else {
            // SAFETY: the branch guarantees `(x-1, y-1)..=(x+1, y+1)` is in
            // bounds for this width and height.
            unsafe { fast_pixel(raw, width, x, y, phase, centre) }
        };
        let offset = x as usize * 3;
        output[offset] = rgb[0];
        output[offset + 1] = rgb[1];
        output[offset + 2] = rgb[2];
    }
}

#[inline(always)]
unsafe fn fast_pixel(
    raw: &[f32],
    width: u32,
    x: u32,
    y: u32,
    phase: BayerPhase,
    centre: f32,
) -> [f32; 3] {
    unsafe {
        let left_right = {
            let left = *raw.get_unchecked(idx(width, x - 1, y));
            let right = *raw.get_unchecked(idx(width, x + 1, y));
            f32::midpoint(left, right)
        };
        let top_down = {
            let top = *raw.get_unchecked(idx(width, x, y - 1));
            let bottom = *raw.get_unchecked(idx(width, x, y + 1));
            f32::midpoint(top, bottom)
        };
        let diagonal = {
            let tl = *raw.get_unchecked(idx(width, x - 1, y - 1));
            let tr = *raw.get_unchecked(idx(width, x + 1, y - 1));
            let bl = *raw.get_unchecked(idx(width, x - 1, y + 1));
            let br = *raw.get_unchecked(idx(width, x + 1, y + 1));
            (tl + tr + bl + br) * 0.25
        };
        let cardinal = {
            let left = *raw.get_unchecked(idx(width, x - 1, y));
            let right = *raw.get_unchecked(idx(width, x + 1, y));
            let top = *raw.get_unchecked(idx(width, x, y - 1));
            let bottom = *raw.get_unchecked(idx(width, x, y + 1));
            (left + right + top + bottom) * 0.25
        };
        phase_with_neighbours(phase, centre, cardinal, left_right, top_down, diagonal)
    }
}

fn checked_pixel(
    raw: &[f32],
    width: u32,
    height: u32,
    x: u32,
    y: u32,
    phase: BayerPhase,
    centre: f32,
) -> [f32; 3] {
    let left_right = average2(
        x.checked_sub(1).map(|left| raw[idx(width, left, y)]),
        (x + 1 < width).then(|| raw[idx(width, x + 1, y)]),
    );
    let top_down = average2(
        y.checked_sub(1).map(|top| raw[idx(width, x, top)]),
        (y + 1 < height).then(|| raw[idx(width, x, y + 1)]),
    );
    let cardinal = average4([
        x.checked_sub(1).map(|left| raw[idx(width, left, y)]),
        (x + 1 < width).then(|| raw[idx(width, x + 1, y)]),
        y.checked_sub(1).map(|top| raw[idx(width, x, top)]),
        (y + 1 < height).then(|| raw[idx(width, x, y + 1)]),
    ]);
    let diagonal = average4([
        x.checked_sub(1)
            .zip(y.checked_sub(1))
            .map(|(left, top)| raw[idx(width, left, top)]),
        (x + 1 < width)
            .then_some(y.checked_sub(1))
            .flatten()
            .map(|top| raw[idx(width, x + 1, top)]),
        (y + 1 < height)
            .then_some(x.checked_sub(1))
            .flatten()
            .map(|left| raw[idx(width, left, y + 1)]),
        (x + 1 < width && y + 1 < height).then(|| raw[idx(width, x + 1, y + 1)]),
    ]);
    phase_with_neighbours(phase, centre, cardinal, left_right, top_down, diagonal)
}

const fn phase_with_neighbours(
    phase: BayerPhase,
    centre: f32,
    cardinal: f32,
    left_right: f32,
    top_down: f32,
    diagonal: f32,
) -> [f32; 3] {
    match phase {
        BayerPhase::Red => [centre, cardinal, diagonal],
        BayerPhase::GreenHorizontalRed => [left_right, centre, top_down],
        BayerPhase::GreenVerticalRed => [top_down, centre, left_right],
        BayerPhase::Blue => [diagonal, cardinal, centre],
    }
}

fn average2(a: Option<f32>, b: Option<f32>) -> f32 {
    match (a, b) {
        (Some(a), Some(b)) => f32::midpoint(a, b),
        (Some(value), None) | (None, Some(value)) => value,
        (None, None) => 0.0,
    }
}

fn average4(values: [Option<f32>; 4]) -> f32 {
    let (sum, count) = values
        .into_iter()
        .flatten()
        .fold((0.0, 0_u8), |(sum, count), value| (sum + value, count + 1));
    if count == 0 {
        0.0
    } else {
        sum / f32::from(count)
    }
}

#[cfg(test)]
mod tests {
    use std::num::{NonZeroU32, NonZeroU64};

    use image::ImageBuffer;

    use super::*;
    use crate::geometry::{ActiveArea, PixelSize, StoredGeometry};
    use crate::metadata::{
        AuthorityClass, BayerLayout, BayerPattern, BlackLevels, CommonMetadata, Confidence,
        DecoderProvenance, ExtensibleMetadata, Provenance, ProviderId, Rational, SourceKind,
        Sourced, ValidatedMetadata, WhiteLevels,
    };
    use crate::{DecodedRaw, RawImage, SampleFormat, SampleSpec, StoredPlane, into_preprocessed};

    fn provenance() -> Provenance {
        Provenance::new(
            ProviderId::try_new("test").unwrap(),
            SourceKind::Derived {
                operation: "fixture".into(),
            },
            AuthorityClass::DerivedValue,
            Confidence::Exact,
        )
    }

    fn sourced<T>(value: T) -> Sourced<T> { Sourced::new(value, provenance()) }

    fn image(pattern: BayerPattern, samples: Vec<f32>) -> RawImage {
        let dimensions = PixelSize::new(3, 3).unwrap();
        let active_area = ActiveArea::try_new(dimensions, 0, 0, 3, 3).unwrap();
        let zero = Rational::new(0, NonZeroU32::new(1).unwrap());
        let white = Rational::new(1, NonZeroU32::new(1).unwrap());
        let common = CommonMetadata {
            sample_spec: Some(sourced(
                SampleSpec::try_new(32, SampleFormat::Float).unwrap(),
            )),
            stored_geometry: Some(sourced(StoredGeometry::new(
                dimensions,
                NonZeroU64::new(12).unwrap(),
            ))),
            active_area: Some(sourced(active_area)),
            cfa: Some(sourced(BayerLayout::new(pattern, active_area))),
            black: Some(sourced(
                BlackLevels::try_new(zero, [zero; 4], None).unwrap(),
            )),
            white: Some(sourced(
                WhiteLevels::try_new(vec![white], None, None).unwrap(),
            )),
            ..CommonMetadata::default()
        };
        let metadata = ValidatedMetadata::try_new(
            common,
            ExtensibleMetadata::default(),
            DecoderProvenance::new(ProviderId::try_new("test").unwrap(), None),
        )
        .unwrap();
        let plane = ImageBuffer::from_raw(3, 3, samples).unwrap();
        let decoded = DecodedRaw::try_new(metadata, StoredPlane::F32(plane.clone())).unwrap();
        into_preprocessed(decoded, plane).unwrap()
    }

    fn run(pattern: BayerPattern) -> Vec<[f32; 3]> {
        let samples = (1_u16..=9).map(|value| f32::from(value) / 16.0).collect();
        let input = image(pattern, samples);
        demosaic(&input)
            .unwrap()
            .pixels()
            .iter()
            .map(|pixel| pixel.0)
            .collect()
    }

    #[test]
    fn reproduces_rggb_linear_result() {
        assert_eq!(
            run(BayerPattern::Rggb),
            vec![
                scaled([1, 3, 5]),
                scaled([2, 2, 5]),
                scaled([3, 4, 5]),
                scaled([4, 4, 5]),
                scaled([5, 5, 5]),
                scaled([6, 6, 5]),
                scaled([7, 6, 5]),
                scaled([8, 8, 5]),
                scaled([9, 7, 5]),
            ]
        );
    }

    #[test]
    fn reproduces_bggr_linear_result() {
        assert_close(
            BayerPattern::Bggr,
            [
                scaled([5, 3, 1]),
                scaled([5, 2, 2]),
                scaled([5, 4, 3]),
                scaled([5, 4, 4]),
                scaled([5, 5, 5]),
                scaled([5, 6, 6]),
                scaled([5, 6, 7]),
                scaled([5, 8, 8]),
                scaled([5, 7, 9]),
            ],
        );
    }

    #[test]
    fn reproduces_grbg_linear_result() {
        assert_close(
            BayerPattern::Grbg,
            [
                scaled([2, 1, 4]),
                scaled([2, 3, 5]),
                scaled([2, 3, 6]),
                scaled_f32([5.0, 13.0 / 3.0, 4.0]),
                scaled([5, 5, 5]),
                scaled_f32([5.0, 17.0 / 3.0, 6.0]),
                scaled([8, 7, 4]),
                scaled([8, 7, 5]),
                scaled([8, 9, 6]),
            ],
        );
    }

    #[test]
    fn reproduces_gbrg_linear_result() {
        assert_close(
            BayerPattern::Gbrg,
            [
                scaled([4, 1, 2]),
                scaled([5, 3, 2]),
                scaled([6, 3, 2]),
                scaled_f32([4.0, 13.0 / 3.0, 5.0]),
                scaled([5, 5, 5]),
                scaled_f32([6.0, 17.0 / 3.0, 5.0]),
                scaled([4, 7, 8]),
                scaled([5, 7, 8]),
                scaled([6, 9, 8]),
            ],
        );
    }

    fn assert_close(pattern: BayerPattern, expected: [[f32; 3]; 9]) {
        for (actual, expected) in run(pattern).into_iter().zip(expected) {
            for (actual, expected) in actual.into_iter().zip(expected) {
                assert!(
                    (actual - expected).abs() < 1e-6,
                    "{pattern:?}: {actual} != {expected}"
                );
            }
        }
    }

    fn scaled(rgb: [u8; 3]) -> [f32; 3] { rgb.map(|value| f32::from(value) / 16.0) }

    fn scaled_f32(rgb: [f32; 3]) -> [f32; 3] { rgb.map(|value| value / 16.0) }

    #[test]
    fn active_area_origin_controls_phase() {
        let dimensions = PixelSize::new(4, 4).unwrap();
        let area = ActiveArea::try_new(dimensions, 1, 1, 4, 4).unwrap();
        let zero = Rational::new(0, NonZeroU32::new(1).unwrap());
        let white = Rational::new(1, NonZeroU32::new(1).unwrap());
        let metadata = ValidatedMetadata::try_new(
            CommonMetadata {
                sample_spec: Some(sourced(
                    SampleSpec::try_new(32, SampleFormat::Float).unwrap(),
                )),
                stored_geometry: Some(sourced(StoredGeometry::new(
                    dimensions,
                    NonZeroU64::new(16).unwrap(),
                ))),
                active_area: Some(sourced(area)),
                cfa: Some(sourced(BayerLayout::new(BayerPattern::Rggb, area))),
                black: Some(sourced(
                    BlackLevels::try_new(zero, [zero; 4], None).unwrap(),
                )),
                white: Some(sourced(
                    WhiteLevels::try_new(vec![white], None, None).unwrap(),
                )),
                ..CommonMetadata::default()
            },
            ExtensibleMetadata::default(),
            DecoderProvenance::new(ProviderId::try_new("test").unwrap(), None),
        )
        .unwrap();
        let plane = ImageBuffer::from_raw(4, 4, vec![0.5; 16]).unwrap();
        let decoded = DecodedRaw::try_new(metadata, StoredPlane::F32(plane.clone())).unwrap();
        let input = into_preprocessed(decoded, plane).unwrap();
        assert!(matches!(
            phase_at(input.metadata().cfa(), 1, 1),
            BayerPhase::Red
        ));
    }

    #[test]
    fn typed_errors_reject_invalid_dimensions() {
        assert_eq!(
            DemosaicError::InvalidInput("bad".into()).to_string(),
            "invalid demosaicing input: bad"
        );
    }
}
