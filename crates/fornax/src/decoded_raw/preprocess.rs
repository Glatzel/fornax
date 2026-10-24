use image::ImageBuffer;

use super::{
    DecodedRaw, F32Plane, SampleFormat, SampleSpec, StoredPlane, U8Plane, U16Plane, U32Plane,
};
use crate::RawImage;
use crate::error::PreprocessError;
use crate::geometry::ActiveArea;
use crate::metadata::{
    BayerLayout, BayerSlot, BlackLevels, BlackTile, ValidatedMetadata, WhiteLevels,
};

/// Runs the ordered standard preprocessing sequence.
pub(super) fn run(image: DecodedRaw) -> Result<RawImage, PreprocessError> {
    image
        .validate_for_preprocessing()
        .map_err(|error| invalid_input(error.to_string()))?;

    let spec = image.sample_spec();
    let (metadata, stored) = image.into_parts();
    let selector = LevelSelector::try_new(&metadata)?;
    let mut plane = into_f32_plane(stored, spec)?;
    linearize(&mut plane, spec, None)?;

    subtract_black(&mut plane, &selector)?;
    normalize(&mut plane, &selector)?;
    plane = crop_to_active_area(plane, metadata.active_area())?;
    finish_for_demosaic(&mut plane)?;

    RawImage::from_preprocessed(metadata, plane).map_err(|error| invalid_input(error.to_string()))
}

fn invalid_input(message: impl Into<Box<str>>) -> PreprocessError {
    PreprocessError::InvalidInput(message.into())
}

fn into_f32_plane(plane: StoredPlane, spec: SampleSpec) -> Result<F32Plane, PreprocessError> {
    match (plane, spec.format()) {
        (StoredPlane::F32(plane), _) => Ok(plane),
        (plane, SampleFormat::Float) => Err(invalid_input(format!(
            "{} storage does not match the declared floating-point sample format",
            plane_name(&plane)
        ))),
        (_plane, SampleFormat::UnsignedInteger) if !spec.is_f32_exact() => {
            Err(PreprocessError::LossyConversion {
                bits_per_sample: spec.bits_per_sample(),
            })
        }
        (StoredPlane::U8(plane), SampleFormat::UnsignedInteger) => {
            convert_u8(plane, spec.bits_per_sample())
        }
        (StoredPlane::U16(plane), SampleFormat::UnsignedInteger) => {
            convert_u16(plane, spec.bits_per_sample())
        }
        (StoredPlane::U32(plane), SampleFormat::UnsignedInteger) => {
            convert_u32(plane, spec.bits_per_sample())
        }
    }
}

fn plane_name(plane: &StoredPlane) -> &'static str {
    match plane {
        StoredPlane::U8(_) => "u8",
        StoredPlane::U16(_) => "u16",
        StoredPlane::U32(_) => "u32",
        StoredPlane::F32(_) => "f32",
    }
}

fn convert_u8(plane: U8Plane, bits_per_sample: u8) -> Result<F32Plane, PreprocessError> {
    if !(1..=8).contains(&bits_per_sample) {
        return Err(invalid_input(
            "u8 storage is incompatible with its declared precision",
        ));
    }
    let dimensions = plane.dimensions();
    collect_f32(dimensions, plane.into_raw().into_iter().map(f32::from))
}

fn convert_u16(plane: U16Plane, bits_per_sample: u8) -> Result<F32Plane, PreprocessError> {
    if !(9..=16).contains(&bits_per_sample) {
        return Err(invalid_input(
            "u16 storage is incompatible with its declared precision",
        ));
    }
    let dimensions = plane.dimensions();
    collect_f32(dimensions, plane.into_raw().into_iter().map(f32::from))
}

fn convert_u32(plane: U32Plane, bits_per_sample: u8) -> Result<F32Plane, PreprocessError> {
    if !(17..=24).contains(&bits_per_sample) {
        return Err(PreprocessError::LossyConversion { bits_per_sample });
    }
    let dimensions = plane.dimensions();
    let source = plane.into_raw();
    let mut samples = Vec::new();
    samples
        .try_reserve_exact(source.len())
        .map_err(|_| PreprocessError::Allocation)?;
    for sample in source {
        let converted = sample as f32;
        if f64::from(converted) != f64::from(sample) {
            return Err(PreprocessError::LossyConversion { bits_per_sample });
        }
        samples.push(converted);
    }
    ImageBuffer::from_raw(dimensions.0, dimensions.1, samples)
        .ok_or_else(|| invalid_input("converted plane dimensions do not match its sample count"))
}

fn collect_f32(
    dimensions: (u32, u32),
    samples: impl ExactSizeIterator<Item = f32>,
) -> Result<F32Plane, PreprocessError> {
    let mut converted = Vec::new();
    converted
        .try_reserve_exact(samples.len())
        .map_err(|_| PreprocessError::Allocation)?;
    converted.extend(samples);
    ImageBuffer::from_raw(dimensions.0, dimensions.1, converted)
        .ok_or_else(|| invalid_input("converted plane dimensions do not match its sample count"))
}

fn linearize(
    plane: &mut F32Plane,
    spec: SampleSpec,
    table: Option<&[f32]>,
) -> Result<(), PreprocessError> {
    let Some(table) = table else {
        return plane
            .as_raw()
            .iter()
            .all(|sample| sample.is_finite())
            .then_some(())
            .ok_or(PreprocessError::Numeric);
    };

    if spec.format() != SampleFormat::UnsignedInteger
        || table.is_empty()
        || table.iter().any(|value| !value.is_finite())
    {
        return Err(PreprocessError::MalformedLinearization);
    }

    for sample in plane.as_mut().iter_mut() {
        if !sample.is_finite() || *sample < 0.0 || sample.fract() != 0.0 {
            return Err(invalid_input(
                "linearization table requires non-negative integer source samples",
            ));
        }
        let code = *sample as u32;
        let replacement =
            table
                .get(code as usize)
                .ok_or(PreprocessError::LinearizationOutOfRange {
                    code,
                    table_len: table.len(),
                })?;
        *sample = *replacement;
    }
    Ok(())
}

struct LevelSelector<'a> {
    black: &'a BlackLevels,
    black_tile: Option<&'a BlackTile>,
    white: &'a WhiteLevels,
    layout: BayerLayout,
}

impl<'a> LevelSelector<'a> {
    fn try_new(metadata: &'a ValidatedMetadata) -> Result<Self, PreprocessError> {
        let black_tile = metadata.black().repeat();
        if let Some(tile) = black_tile {
            let supported_shape =
                matches!((tile.rows().get(), tile.columns().get()), (1, 1) | (2, 2));
            if !supported_shape || tile.samples_per_plane().get() != 1 {
                return Err(PreprocessError::UnsupportedBlackPattern {
                    rows: tile.rows().get(),
                    columns: tile.columns().get(),
                    samples_per_plane: tile.samples_per_plane().get(),
                });
            }
        }
        Ok(Self {
            black: metadata.black(),
            black_tile,
            white: metadata.white(),
            layout: metadata.cfa(),
        })
    }

    fn select(&self, row: u32, column: u32) -> Result<(f64, f64), PreprocessError> {
        let area = self.layout.active_area();
        let slot = slot_at(area, row, column);
        let mut black =
            self.black.common().as_f64() + self.black.per_slot()[slot.index() as usize].as_f64();
        if let Some(tile) = self.black_tile {
            let local_row = i64::from(row).saturating_sub(i64::from(area.rect().top()));
            let local_column = i64::from(column).saturating_sub(i64::from(area.rect().left()));
            let tile_row = local_row.rem_euclid(i64::from(tile.rows().get())) as usize;
            let tile_column = local_column.rem_euclid(i64::from(tile.columns().get())) as usize;
            let index = tile_row * tile.columns().get() as usize + tile_column;
            black += tile.values()[index].as_f64();
        }

        let white = match self.white.declared() {
            [scalar] => scalar.as_f64(),
            values => values[slot.index() as usize].as_f64(),
        };
        let range = white - black;
        if !black.is_finite() || !white.is_finite() || !range.is_finite() || range <= 0.0 {
            return Err(PreprocessError::InvalidLevelRange);
        }
        Ok((black, white))
    }
}

fn slot_at(area: ActiveArea, row: u32, column: u32) -> BayerSlot {
    // Preserve the ActiveArea-relative CFA phase for stored margins too.
    let local_row = (i64::from(row) - i64::from(area.rect().top())).rem_euclid(2) as u32;
    let local_column = (i64::from(column) - i64::from(area.rect().left())).rem_euclid(2) as u32;
    BayerSlot::from_local(local_row, local_column)
}

fn subtract_black(
    plane: &mut F32Plane,
    selector: &LevelSelector<'_>,
) -> Result<(), PreprocessError> {
    for (column, row, pixel) in plane.enumerate_pixels_mut() {
        let (black, _) = selector.select(row, column)?;
        pixel[0] = round_f32(f64::from(pixel[0]) - black)?;
    }
    Ok(())
}

fn normalize(plane: &mut F32Plane, selector: &LevelSelector<'_>) -> Result<(), PreprocessError> {
    for (column, row, pixel) in plane.enumerate_pixels_mut() {
        let (black, white) = selector.select(row, column)?;
        let range = white - black;
        pixel[0] = round_f32(f64::from(pixel[0]) / range)?;
    }
    Ok(())
}

fn finish_for_demosaic(plane: &mut F32Plane) -> Result<(), PreprocessError> {
    for sample in plane.as_mut() {
        if !sample.is_finite() {
            return Err(PreprocessError::Numeric);
        }
        *sample = sample.clamp(0.0, 1.0);
    }
    Ok(())
}

fn crop_to_active_area(
    plane: F32Plane,
    active_area: ActiveArea,
) -> Result<F32Plane, PreprocessError> {
    let rect = active_area.rect();
    let dimensions = plane.dimensions();
    if rect.top() == 0
        && rect.left() == 0
        && rect.width() == dimensions.0
        && rect.height() == dimensions.1
    {
        return Ok(plane);
    }

    let sample_count = usize::try_from(u64::from(rect.width()) * u64::from(rect.height()))
        .map_err(|_| invalid_input("active-area sample count overflows usize"))?;
    let mut samples = Vec::new();
    samples
        .try_reserve_exact(sample_count)
        .map_err(|_| PreprocessError::Allocation)?;
    for row in rect.top()..rect.bottom() {
        for column in rect.left()..rect.right() {
            samples.push(plane.get_pixel(column, row)[0]);
        }
    }
    ImageBuffer::from_raw(rect.width(), rect.height(), samples)
        .ok_or_else(|| invalid_input("cropped plane dimensions do not match its sample count"))
}

fn round_f32(value: f64) -> Result<f32, PreprocessError> {
    let rounded = value as f32;
    rounded
        .is_finite()
        .then_some(rounded)
        .ok_or(PreprocessError::Numeric)
}

#[cfg(test)]
mod tests {
    use std::num::{NonZeroU32, NonZeroU64};
    use std::sync::Arc;

    use super::*;
    use crate::geometry::{PixelSize, StoredGeometry};
    use crate::metadata::{
        AuthorityClass, BayerPattern, CommonMetadata, Confidence, DecoderProvenance,
        ExtensibleMetadata, Provenance, ProviderId, Rational, SourceKind, Sourced,
    };

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

    fn rational(value: i64) -> Rational { Rational::new(value, NonZeroU32::new(1).unwrap()) }

    fn black(common: i64, per_slot: [i64; 4], tile: Option<BlackTile>) -> BlackLevels {
        BlackLevels::try_new(rational(common), per_slot.map(rational), tile).unwrap()
    }

    fn white(values: &[i64]) -> WhiteLevels {
        WhiteLevels::try_new(values.iter().copied().map(rational).collect(), None, None).unwrap()
    }

    fn metadata(
        dimensions: PixelSize,
        active_area: ActiveArea,
        pattern: BayerPattern,
        black: BlackLevels,
        white: WhiteLevels,
        spec: SampleSpec,
    ) -> ValidatedMetadata {
        let common = CommonMetadata {
            sample_spec: Some(sourced(spec)),
            stored_geometry: Some(sourced(StoredGeometry::new(
                dimensions,
                NonZeroU64::new(16).unwrap(),
            ))),
            active_area: Some(sourced(active_area)),
            cfa: Some(sourced(BayerLayout::new(pattern, active_area))),
            black: Some(sourced(black)),
            white: Some(sourced(white)),
            ..CommonMetadata::default()
        };
        ValidatedMetadata::try_new(
            common,
            ExtensibleMetadata::default(),
            DecoderProvenance::new(ProviderId::try_new("test").unwrap(), None),
        )
        .unwrap()
    }

    fn simple_metadata(
        dimensions: PixelSize,
        active_area: ActiveArea,
        pattern: BayerPattern,
        black: BlackLevels,
        white: WhiteLevels,
        format: SampleFormat,
        bits: u8,
    ) -> ValidatedMetadata {
        metadata(
            dimensions,
            active_area,
            pattern,
            black,
            white,
            SampleSpec::try_new(bits, format).unwrap(),
        )
    }

    fn full_area(dimensions: PixelSize) -> ActiveArea {
        ActiveArea::try_new(dimensions, 0, 0, dimensions.height(), dimensions.width()).unwrap()
    }

    #[test]
    fn preprocess_accepts_every_stored_plane() {
        let dimensions = PixelSize::new(1, 1).unwrap();
        let area = full_area(dimensions);
        let cases = [
            (
                SampleSpec::try_new(8, SampleFormat::UnsignedInteger).unwrap(),
                WhiteLevels::try_new(vec![rational(255)], None, None).unwrap(),
                StoredPlane::U8(ImageBuffer::from_raw(1, 1, vec![255]).unwrap()),
                1.0,
            ),
            (
                SampleSpec::try_new(16, SampleFormat::UnsignedInteger).unwrap(),
                WhiteLevels::try_new(vec![rational(65_535)], None, None).unwrap(),
                StoredPlane::U16(ImageBuffer::from_raw(1, 1, vec![65_535]).unwrap()),
                1.0,
            ),
            (
                SampleSpec::try_new(24, SampleFormat::UnsignedInteger).unwrap(),
                WhiteLevels::try_new(vec![rational((1 << 24) - 1)], None, None).unwrap(),
                StoredPlane::U32(ImageBuffer::from_raw(1, 1, vec![(1 << 24) - 1]).unwrap()),
                1.0,
            ),
            (
                SampleSpec::try_new(32, SampleFormat::Float).unwrap(),
                WhiteLevels::try_new(vec![rational(1)], None, None).unwrap(),
                StoredPlane::F32(ImageBuffer::from_raw(1, 1, vec![0.25]).unwrap()),
                0.25,
            ),
        ];

        for (spec, white, plane, expected) in cases {
            let common = metadata(
                dimensions,
                area,
                BayerPattern::Rggb,
                black(0, [0; 4], None),
                white,
                spec,
            );
            let decoded = DecodedRaw::try_new(common, plane).unwrap();
            let output = decoded.preprocess().unwrap();
            assert_eq!(output.plane().get_pixel(0, 0)[0], expected);
        }
    }

    #[test]
    fn wide_u32_rejects_lossy_conversion() {
        let dimensions = PixelSize::new(1, 1).unwrap();
        let area = full_area(dimensions);
        let metadata = simple_metadata(
            dimensions,
            area,
            BayerPattern::Rggb,
            black(0, [0; 4], None),
            white(&[100]),
            SampleFormat::UnsignedInteger,
            32,
        );
        let plane = StoredPlane::U32(ImageBuffer::from_raw(1, 1, vec![1]).unwrap());
        assert_eq!(
            DecodedRaw::try_new(metadata, plane)
                .unwrap()
                .preprocess()
                .unwrap_err(),
            PreprocessError::LossyConversion {
                bits_per_sample: 32
            }
        );

        let metadata = simple_metadata(
            dimensions,
            area,
            BayerPattern::Rggb,
            black(0, [0; 4], None),
            white(&[100]),
            SampleFormat::UnsignedInteger,
            24,
        );
        let plane = StoredPlane::U32(ImageBuffer::from_raw(1, 1, vec![16_777_217]).unwrap());
        assert_eq!(
            DecodedRaw::try_new(metadata, plane)
                .unwrap()
                .preprocess()
                .unwrap_err(),
            PreprocessError::LossyConversion {
                bits_per_sample: 24
            }
        );
    }

    #[test]
    fn identity_and_lookup_linearization_validate_the_source_domain() {
        let spec = SampleSpec::try_new(8, SampleFormat::UnsignedInteger).unwrap();
        let mut plane = ImageBuffer::from_raw(2, 1, vec![0.0, 2.0]).unwrap();
        linearize(&mut plane, spec, None).unwrap();
        assert_eq!(plane.as_raw(), &[0.0, 2.0]);

        linearize(&mut plane, spec, Some(&[0.25, 0.5, 0.75])).unwrap();
        assert_eq!(plane.as_raw(), &[0.25, 0.75]);

        assert_eq!(
            linearize(&mut plane, spec, Some(&[])).unwrap_err(),
            PreprocessError::MalformedLinearization
        );
        assert_eq!(
            linearize(&mut plane, spec, Some(&[0.0, f32::NAN])).unwrap_err(),
            PreprocessError::MalformedLinearization
        );

        plane.as_mut()[0] = 0.5;
        assert!(matches!(
            linearize(&mut plane, spec, Some(&[0.0, 1.0])),
            Err(PreprocessError::InvalidInput(_))
        ));
        plane.as_mut()[0] = 2.0;
        assert_eq!(
            linearize(&mut plane, spec, Some(&[0.0, 1.0])).unwrap_err(),
            PreprocessError::LinearizationOutOfRange {
                code: 2,
                table_len: 2
            }
        );
    }

    #[test]
    fn black_selection_uses_all_bayer_slots_and_repeat_tiles() {
        let dimensions = PixelSize::new(4, 4).unwrap();
        let area = full_area(dimensions);
        let tile = BlackTile::try_new(
            NonZeroU32::new(2).unwrap(),
            NonZeroU32::new(2).unwrap(),
            NonZeroU32::new(1).unwrap(),
            vec![rational(8), rational(9), rational(10), rational(11)],
        )
        .unwrap();
        let black = black(10, [0, 1, 2, 3], Some(tile));
        for pattern in [
            BayerPattern::Rggb,
            BayerPattern::Bggr,
            BayerPattern::Grbg,
            BayerPattern::Gbrg,
        ] {
            let metadata = simple_metadata(
                dimensions,
                area,
                pattern,
                black.clone(),
                white(&[100]),
                SampleFormat::Float,
                32,
            );
            let selector = LevelSelector::try_new(&metadata).unwrap();
            for row in 0..2 {
                for column in 0..2 {
                    let slot = slot_at(area, row, column);
                    let repeat = [8.0, 9.0, 10.0, 11.0][row as usize * 2 + column as usize];
                    assert_eq!(
                        selector.select(row, column).unwrap().0,
                        10.0 + f64::from(slot.index()) + repeat,
                        "{pattern:?} at ({row}, {column})"
                    );
                }
            }
        }
    }

    #[test]
    fn repeat_selection_is_relative_to_an_odd_active_area_origin() {
        let dimensions = PixelSize::new(4, 4).unwrap();
        let area = ActiveArea::try_new(dimensions, 1, 1, 4, 4).unwrap();
        let tile = BlackTile::try_new(
            NonZeroU32::new(2).unwrap(),
            NonZeroU32::new(2).unwrap(),
            NonZeroU32::new(1).unwrap(),
            vec![rational(20), rational(21), rational(22), rational(23)],
        )
        .unwrap();
        let metadata = simple_metadata(
            dimensions,
            area,
            BayerPattern::Gbrg,
            black(0, [0; 4], Some(tile)),
            white(&[100]),
            SampleFormat::Float,
            32,
        );
        let selector = LevelSelector::try_new(&metadata).unwrap();
        assert_eq!(selector.select(1, 1).unwrap().0, 20.0);
        assert_eq!(selector.select(2, 1).unwrap().0, 22.0);
        assert_eq!(selector.select(1, 2).unwrap().0, 21.0);
        assert_eq!(selector.select(2, 2).unwrap().0, 23.0);
    }

    #[test]
    fn one_by_one_repeat_is_supported() {
        let dimensions = PixelSize::new(2, 2).unwrap();
        let area = full_area(dimensions);
        let tile = BlackTile::try_new(
            NonZeroU32::new(1).unwrap(),
            NonZeroU32::new(1).unwrap(),
            NonZeroU32::new(1).unwrap(),
            vec![rational(7)],
        )
        .unwrap();
        let metadata = simple_metadata(
            dimensions,
            area,
            BayerPattern::Rggb,
            black(1, [0; 4], Some(tile)),
            white(&[100]),
            SampleFormat::Float,
            32,
        );
        let selector = LevelSelector::try_new(&metadata).unwrap();
        assert_eq!(selector.select(1, 1).unwrap().0, 8.0);
    }

    #[test]
    fn unsupported_black_decompositions_are_typed() {
        let dimensions = PixelSize::new(2, 2).unwrap();
        let area = full_area(dimensions);
        let tile = BlackTile::try_new(
            NonZeroU32::new(3).unwrap(),
            NonZeroU32::new(1).unwrap(),
            NonZeroU32::new(1).unwrap(),
            vec![rational(8), rational(9), rational(10)],
        )
        .unwrap();
        let metadata = simple_metadata(
            dimensions,
            area,
            BayerPattern::Rggb,
            black(0, [0; 4], Some(tile)),
            white(&[100]),
            SampleFormat::Float,
            32,
        );
        assert!(matches!(
            LevelSelector::try_new(&metadata),
            Err(PreprocessError::UnsupportedBlackPattern {
                rows: 3,
                columns: 1,
                samples_per_plane: 1
            })
        ));

        let tile = BlackTile::try_new(
            NonZeroU32::new(1).unwrap(),
            NonZeroU32::new(2).unwrap(),
            NonZeroU32::new(1).unwrap(),
            vec![rational(8), rational(9)],
        )
        .unwrap();
        let metadata = simple_metadata(
            dimensions,
            area,
            BayerPattern::Rggb,
            black(0, [0; 4], Some(tile)),
            white(&[100]),
            SampleFormat::Float,
            32,
        );
        assert!(matches!(
            LevelSelector::try_new(&metadata),
            Err(PreprocessError::UnsupportedBlackPattern {
                rows: 1,
                columns: 2,
                samples_per_plane: 1
            })
        ));
    }

    #[test]
    fn subtraction_and_normalization_preserve_signed_range_until_clipping() {
        let dimensions = PixelSize::new(2, 1).unwrap();
        let area = full_area(dimensions);
        let metadata = simple_metadata(
            dimensions,
            area,
            BayerPattern::Rggb,
            black(10, [0; 4], None),
            white(&[20]),
            SampleFormat::Float,
            32,
        );
        let selector = LevelSelector::try_new(&metadata).unwrap();
        let mut plane = ImageBuffer::from_raw(2, 1, vec![5.0, 30.0]).unwrap();

        subtract_black(&mut plane, &selector).unwrap();
        assert_eq!(plane.as_raw(), &[-5.0, 20.0]);
        normalize(&mut plane, &selector).unwrap();
        assert_eq!(plane.as_raw(), &[-0.5, 2.0]);
        finish_for_demosaic(&mut plane).unwrap();
        assert_eq!(plane.as_raw(), &[0.0, 1.0]);
    }

    #[test]
    fn preprocessing_retains_metadata_and_active_area() {
        let dimensions = PixelSize::new(2, 2).unwrap();
        let area = ActiveArea::try_new(dimensions, 1, 1, 2, 2).unwrap();
        let metadata = simple_metadata(
            dimensions,
            area,
            BayerPattern::Gbrg,
            black(0, [0; 4], None),
            white(&[1]),
            SampleFormat::Float,
            32,
        );
        let decoded = DecodedRaw::try_new(
            metadata,
            StoredPlane::F32(ImageBuffer::from_raw(2, 2, vec![0.5; 4]).unwrap()),
        )
        .unwrap();
        let metadata_arc: Arc<_> = decoded.metadata_arc();
        let output = decoded.preprocess().unwrap();
        assert!(Arc::ptr_eq(&metadata_arc, &output.metadata_arc()));
        assert_eq!(output.metadata().active_area(), area);
        assert_eq!(output.metadata().cfa().pattern(), BayerPattern::Gbrg);
        assert_eq!(output.plane().dimensions(), (1, 1));
    }
}
