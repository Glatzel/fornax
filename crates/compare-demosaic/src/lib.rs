//! Demosaic quality comparison runner over synthetic float32 Bayer TIFFs.

use std::fs;
use std::path::{Path, PathBuf};

use clap::{Parser, ValueEnum};
use clap_verbosity_flag::Verbosity;
use fornax_raw::demosaic::{Demosaic, IDemosaic, LazyDemosaic};
use fornax_raw::{BayerPattern, MonoImage, RgbImage};
use fornax_tiff::image::{TiffImage, TypedTiffImage};
use fornax_tiff::writer::TiffWriteOptions;

/// Production-ready demosaicing methods supported by this comparison harness.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProductionMethod {
    /// Bilinear baseline demosaicing.
    Linear,
    /// Modern LibRaw/darktable patterned pixel grouping (PPG).
    Ppg,
}

impl ProductionMethod {
    /// All production-ready demosaicing methods in canonical order.
    pub const ALL: [Self; 2] = [Self::Linear, Self::Ppg];

    /// Stable lowercase identifier used for directory names and reports.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Linear => "linear",
            Self::Ppg => "ppg",
        }
    }

    /// Converts to `fornax_raw::demosaic::Demosaic`.
    pub const fn to_demosaic(self) -> Demosaic {
        match self {
            Self::Linear => Demosaic::Linear,
            Self::Ppg => Demosaic::Ppg,
        }
    }
}

/// CLI selector for demosaicing methods.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum MethodSelection {
    /// Run all production-ready methods (`linear`, `ppg`).
    All,
    /// Run bilinear demosaicing (`linear`).
    Linear,
    /// Run patterned pixel grouping (`ppg`).
    Ppg,
}

/// CLI arguments for `compare-demosaic`.
#[derive(Debug, Parser)]
#[command(
    name = "compare-demosaic",
    about = "Run production-ready Fornax demosaicing methods on synthetic float32 Bayer TIFFs.",
    styles = clap_style::styles(),
)]
pub struct CliArgs {
    /// Demosaicing method(s) to run (`linear`, `ppg`, or `all`).
    #[arg(long, value_enum, value_delimiter = ',', default_value = "all")]
    pub method: Vec<MethodSelection>,

    /// Directory containing `bayer_{pattern}.tif` inputs.
    /// Defaults to `<repo-root>/temp/compare-demosaic` via
    /// `fornax_devtool::temp_dir()`.
    #[arg(long)]
    pub input_dir: Option<PathBuf>,

    /// Directory where `{method}/demosaic_{pattern}.tif` outputs are written.
    /// Defaults to `--input-dir`.
    #[arg(long)]
    pub output_dir: Option<PathBuf>,

    /// Verbosity level.
    #[command(flatten)]
    pub verbose: Verbosity,
}

/// Outcome record for one `(method, pattern)` demosaic execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DemosaicRun {
    /// Method executed.
    pub method: ProductionMethod,
    /// Bayer pattern processed.
    pub pattern: BayerPattern,
    /// Path to the written float32 RGB TIFF.
    pub output_path: PathBuf,
    /// Image width in pixels.
    pub width: u32,
    /// Image height in pixels.
    pub height: u32,
}

/// Resolves CLI method selections into a deduplicated list of
/// [`ProductionMethod`]s.
pub fn resolve_methods(selections: &[MethodSelection]) -> Vec<ProductionMethod> {
    if selections.is_empty() || selections.contains(&MethodSelection::All) {
        return ProductionMethod::ALL.to_vec();
    }
    let mut resolved = Vec::new();
    for sel in selections {
        let method = match sel {
            MethodSelection::All => unreachable!(),
            MethodSelection::Linear => ProductionMethod::Linear,
            MethodSelection::Ppg => ProductionMethod::Ppg,
        };
        if !resolved.contains(&method) {
            resolved.push(method);
        }
    }
    resolved
}

/// Returns the default temporary directory `<repo-root>/temp/compare-demosaic`.
pub fn default_temp_dir() -> PathBuf { fornax_devtool::temp_dir().join("compare-demosaic") }

/// Returns the canonical lowercase Bayer pattern filename stem (e.g. `rggb`).
pub const fn pattern_slug(pattern: BayerPattern) -> &'static str {
    match pattern {
        BayerPattern::RGGB => "rggb",
        BayerPattern::BGGR => "bggr",
        BayerPattern::GRBG => "grbg",
        BayerPattern::GBRG => "gbrg",
    }
}

/// Reads a single-channel `f32` Bayer TIFF, runs `method`, and writes a
/// 3-channel `f32` RGB TIFF.
pub fn demosaic_tiff(
    input_path: &Path,
    output_path: &Path,
    pattern: BayerPattern,
    method: ProductionMethod,
) -> mischief::Result<(u32, u32)> {
    let image = TiffImage::open(input_path)?;
    let mono: MonoImage<f32> = match image {
        TiffImage::F32(TypedTiffImage::Mono(mono)) => mono,
        other => {
            mischief::bail!(
                "expected 1-channel f32 mono TIFF at {}, found {}ch",
                input_path.display(),
                other.samples_per_pixel()
            );
        }
    };

    let (width, height) = mono.dimensions();
    let rgb: RgbImage<f32> = method.to_demosaic().demosaic(&mono, pattern)?.collect();

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    TiffImage::from(rgb).save(output_path, &TiffWriteOptions::default())?;

    Ok((width, height))
}

/// Executes all selected demosaicing methods across all 4 Bayer patterns.
pub fn run(args: &CliArgs) -> mischief::Result<Vec<DemosaicRun>> {
    let input_dir = args.input_dir.clone().unwrap_or_else(default_temp_dir);
    let output_dir = args.output_dir.clone().unwrap_or_else(|| input_dir.clone());
    let methods = resolve_methods(&args.method);
    let mut outcomes = Vec::with_capacity(methods.len() * BayerPattern::ALL.len());

    for method in methods {
        for pattern in BayerPattern::ALL {
            let slug = pattern_slug(pattern);
            let input_path = input_dir.join(format!("bayer_{slug}.tif"));
            let output_path = output_dir
                .join(method.as_str())
                .join(format!("demosaic_{slug}.tif"));
            let (width, height) = demosaic_tiff(&input_path, &output_path, pattern, method)?;
            outcomes.push(DemosaicRun {
                method,
                pattern,
                output_path,
                width,
                height,
            });
        }
    }

    Ok(outcomes)
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn resolve_methods_handles_all_and_dedup() {
        assert_eq!(
            resolve_methods(&[MethodSelection::All]),
            vec![ProductionMethod::Linear, ProductionMethod::Ppg]
        );
        assert_eq!(
            resolve_methods(&[MethodSelection::Ppg, MethodSelection::Ppg]),
            vec![ProductionMethod::Ppg]
        );
        assert_eq!(
            resolve_methods(&[MethodSelection::Ppg, MethodSelection::Linear]),
            vec![ProductionMethod::Ppg, ProductionMethod::Linear]
        );
    }

    #[rstest]
    #[case(ProductionMethod::Linear, BayerPattern::RGGB)]
    #[case(ProductionMethod::Linear, BayerPattern::BGGR)]
    #[case(ProductionMethod::Linear, BayerPattern::GRBG)]
    #[case(ProductionMethod::Linear, BayerPattern::GBRG)]
    #[case(ProductionMethod::Ppg, BayerPattern::RGGB)]
    #[case(ProductionMethod::Ppg, BayerPattern::BGGR)]
    #[case(ProductionMethod::Ppg, BayerPattern::GRBG)]
    #[case(ProductionMethod::Ppg, BayerPattern::GBRG)]
    fn demosaic_tiff_preserves_unclamped_float_range(
        #[case] method: ProductionMethod,
        #[case] pattern: BayerPattern,
    ) -> mischief::Result<()> {
        let dir = tempdir()?;
        let width = 32_u32;
        let height = 32_u32;
        let mut samples = Vec::with_capacity((width * height) as usize);
        for y in 0..height {
            for x in 0..width {
                // Mix negative (-0.5) and HDR (3.5) values so clamping bugs
                // fail immediately.
                let val = if x < width / 2 { -0.5_f32 } else { 3.5_f32 };
                let _ = y;
                samples.push(val);
            }
        }
        let mono = MonoImage::from_raw(width, height, samples).unwrap();
        let in_path = dir.path().join("bayer.tif");
        let out_path = dir.path().join("rgb.tif");
        TiffImage::from(mono).save(&in_path, &TiffWriteOptions::default())?;

        let (out_w, out_h) = demosaic_tiff(&in_path, &out_path, pattern, method)?;
        assert_eq!((out_w, out_h), (width, height));

        let decoded = match TiffImage::open(&out_path)? {
            TiffImage::F32(TypedTiffImage::Rgb(rgb)) => rgb,
            other => mischief::bail!("unexpected output image variant: {other:?}"),
        };

        let left = decoded.get_pixel(6, 16).0;
        let right = decoded.get_pixel(26, 16).0;
        for ch in left {
            assert!((ch + 0.5).abs() < 1e-4, "expected -0.5, got {ch}");
        }
        for ch in right {
            assert!((ch - 3.5).abs() < 1e-4, "expected 3.5, got {ch}");
        }
        Ok(())
    }
}
