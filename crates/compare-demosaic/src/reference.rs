use std::path::Path;
use std::sync::Mutex;

use fornax_raw_core::protocol::{IDecoder, IPreprocessor};
use fornax_raw_core::{BayerChannel, BayerPattern, CroppedMosaic};
use fornax_raw_decoder::Decoder;
use fornax_raw_pipeline::Preprocessor;
use sha2::{Digest, Sha256};

use crate::cases::ReferenceBackend;

include!(concat!(env!("OUT_DIR"), "/status_codes.rs"));

static REFERENCE_LOCK: Mutex<()> = Mutex::new(());

#[derive(Clone, Debug)]
pub struct ActiveArea {
    pub source: String,
    pub sha256: String,
    pub width: u32,
    pub height: u32,
    pub codes: Vec<u16>,
    pub black: Vec<f64>,
    pub black_width: u32,
    pub black_height: u32,
    pub white: f64,
    pub native_pattern: BayerPattern,
}

/// Returns the smallest non-negative crop offset that turns `native` into
/// `target` according to [`BayerPattern::shifted`].
pub fn pattern_offset(native: BayerPattern, target: BayerPattern) -> Option<(u32, u32)> {
    (0..2).find_map(|y| {
        (0..2)
            .find(|x| native.shifted(*x, y) == target)
            .map(|x| (x, y))
    })
}

/// Crops one pixel when necessary so that `target` is a valid Bayer label for
/// the returned mosaic.
pub fn shifted_to_pattern(
    input: &ActiveArea,
    target: BayerPattern,
) -> mischief::Result<ActiveArea> {
    let (x, y) = pattern_offset(input.native_pattern, target).ok_or_else(|| {
        mischief::mischief!(
            "no Bayer crop converts {} to {target}",
            input.native_pattern
        )
    })?;
    if x >= input.width || y >= input.height {
        return Err(mischief::mischief!(
            "cannot shift {}x{} input by ({x}, {y})",
            input.width,
            input.height
        ));
    }
    let expected = (input.width as usize)
        .checked_mul(input.height as usize)
        .ok_or_else(|| mischief::mischief!("input dimensions overflow"))?;
    if input.codes.len() != expected {
        return Err(mischief::mischief!(
            "input contains {} samples, expected {expected}",
            input.codes.len()
        ));
    }
    let width = input.width - x;
    let height = input.height - y;
    let codes = (y..input.height)
        .flat_map(|row| {
            let start = row as usize * input.width as usize + x as usize;
            input.codes[start..start + width as usize].iter().copied()
        })
        .collect();
    Ok(ActiveArea {
        source: input.source.clone(),
        sha256: input.sha256.clone(),
        width,
        height,
        codes,
        black: input.black.clone(),
        black_width: input.black_width,
        black_height: input.black_height,
        white: input.white,
        native_pattern: target,
    })
}

pub fn active_area_from(path: &Path) -> mischief::Result<ActiveArea> {
    let source_bytes = std::fs::read(path)
        .map_err(|error| mischief::mischief!("cannot read {}: {error}", path.display()))?;
    let sha256 = hex::encode(Sha256::digest(&source_bytes));
    let decoder: Decoder = Decoder::decode(path)?;
    let decoded: CroppedMosaic<f32> = decoder.cropped_mosaic()?;
    let metadata = decoder.process_metadata()?;
    let image = Preprocessor
        .process(decoded, &metadata.levels)
        .map_err(|error| mischief::mischief!("cannot preprocess {}: {error}", path.display()))?
        .into_image();
    let (width, height) = image.dimensions();
    let (black_width, black_height) = metadata.levels.black.dimensions();
    Ok(ActiveArea {
        source: path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        sha256,
        width,
        height,
        codes: image
            .as_raw()
            .iter()
            .map(|sample| (sample.clamp(0.0, 1.0) * f32::from(u16::MAX)).round() as u16)
            .collect(),
        black: metadata.levels.black.as_raw().clone(),
        black_width,
        black_height,
        white: metadata.levels.white,
        native_pattern: metadata.bayer_pattern,
    })
}

#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Ok = COMPARE_DEMOSAIC_STATUS_OK,
    InvalidDimensions = COMPARE_DEMOSAIC_STATUS_INVALID_DIMENSIONS,
    DimensionOverflow = COMPARE_DEMOSAIC_STATUS_DIMENSION_OVERFLOW,
    AllocationFailed = COMPARE_DEMOSAIC_STATUS_ALLOCATION_FAILED,
    NullPointer = COMPARE_DEMOSAIC_STATUS_NULL_POINTER,
    OutOfMemory = COMPARE_DEMOSAIC_STATUS_OUT_OF_MEMORY,
    Exception = COMPARE_DEMOSAIC_STATUS_EXCEPTION,
    UnknownAlgorithm = COMPARE_DEMOSAIC_STATUS_UNKNOWN_ALGORITHM,
}

impl Status {
    fn from_code(code: i32) -> Self {
        match code {
            COMPARE_DEMOSAIC_STATUS_OK => Self::Ok,
            COMPARE_DEMOSAIC_STATUS_INVALID_DIMENSIONS => Self::InvalidDimensions,
            COMPARE_DEMOSAIC_STATUS_DIMENSION_OVERFLOW => Self::DimensionOverflow,
            COMPARE_DEMOSAIC_STATUS_ALLOCATION_FAILED => Self::AllocationFailed,
            COMPARE_DEMOSAIC_STATUS_NULL_POINTER => Self::NullPointer,
            COMPARE_DEMOSAIC_STATUS_OUT_OF_MEMORY => Self::OutOfMemory,
            COMPARE_DEMOSAIC_STATUS_EXCEPTION => Self::Exception,
            COMPARE_DEMOSAIC_STATUS_UNKNOWN_ALGORITHM => Self::UnknownAlgorithm,
            other => panic!("unknown compare-demosaic FFI status {other}"),
        }
    }

    const fn message(self) -> &'static str {
        match self {
            Self::Ok => "success",
            Self::InvalidDimensions => "reference requires dimensions of at least 3x3",
            Self::DimensionOverflow => "reference dimensions exceed supported limits",
            Self::AllocationFailed => "reference image allocation failed",
            Self::NullPointer => "reference received a null buffer",
            Self::OutOfMemory => "reference ran out of memory",
            Self::Exception => "reference raised an unknown exception",
            Self::UnknownAlgorithm => "reference returned unknown algorithm",
        }
    }
}

unsafe extern "C" {
    fn compare_demosaic_reference(
        reference: u32,
        algorithm: u32,
        mosaic: *const u16,
        width: u32,
        height: u32,
        filters: u32,
        output: *mut u16,
    ) -> i32;
}

pub fn run(
    reference: ReferenceBackend,
    algorithm: u32,
    codes: &[u16],
    width: u32,
    height: u32,
    filters: u32,
) -> mischief::Result<Vec<u16>> {
    let pixels = (width as usize)
        .checked_mul(height as usize)
        .ok_or_else(|| mischief::mischief!("synthetic image dimensions overflow"))?;
    if codes.len() != pixels {
        return Err(mischief::mischief!(
            "synthetic image contains {} samples, expected {pixels}",
            codes.len()
        ));
    }
    let samples = pixels
        .checked_mul(3)
        .ok_or_else(|| mischief::mischief!("RGB output dimensions overflow"))?;
    let mut output = vec![0_u16; samples];

    let _guard = REFERENCE_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    // SAFETY: both buffers are valid for the dimensions checked above, and the
    // C++ function writes exactly `pixels * 3` samples.
    let code = unsafe {
        compare_demosaic_reference(
            match reference {
                ReferenceBackend::LibRaw => 0,
                ReferenceBackend::Darktable => 1,
                ReferenceBackend::RawTherapee => 2,
            },
            algorithm,
            codes.as_ptr(),
            width,
            height,
            filters,
            output.as_mut_ptr(),
        )
    };
    let status = Status::from_code(code);
    if status == Status::Ok {
        Ok(output)
    } else {
        Err(mischief::mischief!("{}", status.message()))
    }
}

pub fn libraw_filters(pattern: BayerPattern) -> u32 {
    let mut filters = 0_u32;
    for y in 0..8 {
        for x in 0..2 {
            let code = match pattern.channel_at(x, y) {
                BayerChannel::R => 0_u32,
                BayerChannel::GRedRow => 1,
                BayerChannel::B => 2,
                BayerChannel::GBlueRow => 3,
            };
            filters |= code << ((y * 2 + x) * 2);
        }
    }
    filters
}

#[cfg(test)]
mod tests {
    use fornax_raw_core::MonoImage;
    use fornax_raw_pipeline::{Demosaic, IDemosaic, LazyDemosaic};

    use super::*;

    #[test]
    fn filter_masks_match_all_bayer_tiles() {
        for pattern in BayerPattern::ALL {
            let filters = libraw_filters(pattern);
            for y in 0..8 {
                for x in 0..2 {
                    let shift = (y * 2 + x) * 2;
                    let code = (filters >> shift) & 3;
                    let channel = match code {
                        0 => BayerChannel::R,
                        1 => BayerChannel::GRedRow,
                        2 => BayerChannel::B,
                        _ => BayerChannel::GBlueRow,
                    };
                    assert_eq!(channel, pattern.channel_at(x, y));
                }
            }
        }
    }

    #[test]
    fn every_target_pattern_has_a_minimal_valid_crop() {
        for native in BayerPattern::ALL {
            for target in BayerPattern::ALL {
                let (x, y) = pattern_offset(native, target).expect("target pattern is reachable");
                assert!(x < 2 && y < 2);
                assert_eq!(native.shifted(x, y), target);
            }
        }
    }

    #[test]
    fn gbrg_offsets_match_the_valid_crop_table() {
        assert_eq!(
            pattern_offset(BayerPattern::GBRG, BayerPattern::GBRG),
            Some((0, 0))
        );
        assert_eq!(
            pattern_offset(BayerPattern::GBRG, BayerPattern::BGGR),
            Some((1, 0))
        );
        assert_eq!(
            pattern_offset(BayerPattern::GBRG, BayerPattern::RGGB),
            Some((0, 1))
        );
        assert_eq!(
            pattern_offset(BayerPattern::GBRG, BayerPattern::GRBG),
            Some((1, 1))
        );
    }

    #[test]
    fn amaze_matches_both_references_across_partial_tiles() {
        let width = 300_u32;
        let height = 300_u32;
        let codes = (0..height)
            .flat_map(|y| {
                (0..width).map(move |x| {
                    if (64..192).contains(&x) && (64..192).contains(&y) {
                        return u16::MAX;
                    }
                    let hash =
                        x.wrapping_mul(0x9e37_79b9).rotate_left(13) ^ y.wrapping_mul(0x85eb_ca6b);
                    (hash ^ (hash >> 16)) as u16
                })
            })
            .collect::<Vec<_>>();
        let filters = libraw_filters(BayerPattern::GBRG);
        let darktable = run(
            ReferenceBackend::Darktable,
            3,
            &codes,
            width,
            height,
            filters,
        )
        .unwrap();
        let rawtherapee = run(
            ReferenceBackend::RawTherapee,
            0,
            &codes,
            width,
            height,
            filters,
        )
        .unwrap();
        let samples = codes
            .iter()
            .map(|value| f32::from(*value) / f32::from(u16::MAX))
            .collect::<Vec<_>>();
        let image = MonoImage::from_raw(width, height, samples).unwrap();
        let actual = Demosaic::Amaze
            .demosaic(&image, BayerPattern::GBRG)
            .unwrap()
            .collect();
        let actual = actual
            .as_raw()
            .iter()
            .map(|value| (value.clamp(0.0, 1.0) * f32::from(u16::MAX)).round() as u16)
            .collect::<Vec<_>>();
        for (name, reference) in [("darktable", darktable), ("RawTherapee", rawtherapee)] {
            let total = reference
                .iter()
                .zip(&actual)
                .map(|(expected, got)| u64::from(expected.abs_diff(*got)))
                .sum::<u64>();
            let maximum = reference
                .iter()
                .zip(&actual)
                .map(|(expected, got)| expected.abs_diff(*got))
                .max()
                .unwrap();
            assert!(
                total as f64 / reference.len() as f64 <= 0.01,
                "AMaZE synthetic {name} MAE is too large"
            );
            assert!(
                maximum <= 2,
                "AMaZE synthetic {name} maximum error is {maximum}"
            );
        }
    }

    #[test]
    fn shifted_crop_rewrites_pattern_and_samples() -> mischief::Result<()> {
        let input = ActiveArea {
            source: "test.raw".to_owned(),
            sha256: "00".repeat(32),
            width: 4,
            height: 4,
            codes: (0..16).collect(),
            black: vec![0.0; 4],
            black_width: 2,
            black_height: 2,
            white: 65_535.0,
            native_pattern: BayerPattern::GBRG,
        };
        let shifted = shifted_to_pattern(&input, BayerPattern::RGGB)?;
        assert_eq!(shifted.width, 4);
        assert_eq!(shifted.height, 3);
        assert_eq!(shifted.native_pattern, BayerPattern::RGGB);
        assert_eq!(shifted.codes, (4..16).collect::<Vec<_>>());
        Ok(())
    }

    #[test]
    fn reference_rejects_invalid_and_overflowing_dimensions() {
        let mosaic = [0_u16; 9];
        let mut output = [0_u16; 27];
        for (width, height, expected) in [
            (2, 3, Status::InvalidDimensions),
            (3, 2, Status::InvalidDimensions),
            (65_536, 3, Status::DimensionOverflow),
            (3, 65_536, Status::DimensionOverflow),
        ] {
            // SAFETY: the C++ function rejects these dimensions before reading
            // either buffer, and both pointers remain valid for the call.
            let code = unsafe {
                compare_demosaic_reference(
                    0,
                    0,
                    mosaic.as_ptr(),
                    width,
                    height,
                    libraw_filters(BayerPattern::RGGB),
                    output.as_mut_ptr(),
                )
            };
            assert_eq!(Status::from_code(code), expected);
        }
    }
}
