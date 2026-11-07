use std::fs;
use std::path::{Path, PathBuf};

use clap::{Parser, ValueEnum};
use clap_verbosity_flag::InfoLevel;
use fornax::dng::DngCompression;
use fornax::postprocessor::Demosaic;
use serde::Deserialize;
use thiserror::Error;

/// Command-line arguments for the Fornax converter.
#[derive(Debug, Parser)]
#[command(about = "Convert RAW images with the Fornax pipeline",author = "Glatzel", styles=clap_style::styles())]
pub struct CliArgs {
    /// TOML or JSON configuration file.
    #[arg(long, help_heading = "General")]
    pub config: Option<PathBuf>,
    /// Input RAW files or directories.
    #[arg(help_heading = "General")]
    pub inputs: Vec<PathBuf>,

    /// Demosaic algorithm with its default parameters [default: rcd]
    #[arg(long, help_heading = "Processing")]
    pub demosaic: Option<String>,

    /// Apply the derived camera-to-linear-sRGB matrix [default: true]
    #[arg(long, help_heading = "Color")]
    pub color_matrix: bool,

    /// Output directory [default: converted]
    #[arg(long, short, help_heading = "Output")]
    pub output: Option<PathBuf>,

    /// Output image format [default: dng]
    #[arg(long, value_enum, help_heading = "Output")]
    pub format: Option<OutputFormat>,

    /// Overwrite an existing output file [default: false]
    #[arg(long, help_heading = "Output")]
    pub overwrite: bool,

    /// Write `<output-stem>.json` beside each output [default: false]
    #[arg(long, help_heading = "Output")]
    pub metadata_json: bool,

    /// JPEG preview quality [default: 90]
    #[arg(long, help_heading = "Output")]
    pub jpeg_quality: Option<u8>,

    /// DNG compression strategy [default: lossless-jpeg]
    #[arg(long, value_enum, help_heading = "Output")]
    pub dng_compression: Option<DngCompressionChoice>,

    /// Experimental Zstandard compression level [default: 3]
    #[arg(long, help_heading = "Output")]
    pub zstd_level: Option<i32>,

    /// Maximum simultaneous file conversions [default: 1]
    #[arg(long, help_heading = "Performance")]
    pub jobs: Option<usize>,

    /// Recurse into directory inputs [default: true]
    #[arg(long, help_heading = "Input")]
    pub recursive: bool,

    #[command(flatten)]
    pub verbose: clap_verbosity_flag::Verbosity<InfoLevel>,
}

/// Output image format.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum OutputFormat {
    #[default]
    Dng,
    Tiff,
    Jpeg,
}

impl OutputFormat {
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Dng => "dng",
            Self::Tiff => "tiff",
            Self::Jpeg => "jpg",
        }
    }
}

/// DNG writer compression selection.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum DngCompressionChoice {
    Uncompressed,
    #[default]
    LosslessJpeg,
    Deflate,
    Zstd,
}

impl DngCompressionChoice {
    pub const fn writer(self) -> DngCompression {
        match self {
            Self::Uncompressed => DngCompression::Uncompressed,
            Self::LosslessJpeg => DngCompression::LosslessJpeg,
            Self::Deflate => DngCompression::Deflate,
            Self::Zstd => DngCompression::Zstd,
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileConfig {
    inputs: Option<Vec<PathBuf>>,
    output: Option<PathBuf>,
    format: Option<OutputFormat>,
    jobs: Option<usize>,
    recursive: Option<bool>,
    overwrite: Option<bool>,
    metadata_json: Option<bool>,
    demosaic: Option<String>,
    color_matrix: Option<bool>,
    jpeg_quality: Option<u8>,
    dng_compression: Option<DngCompressionChoice>,
    zstd_level: Option<i32>,
}

/// Fully resolved converter configuration.
#[derive(Clone, Debug)]
pub struct ResolvedConfig {
    pub inputs: Vec<PathBuf>,
    pub output: PathBuf,
    pub format: OutputFormat,
    pub jobs: usize,
    pub recursive: bool,
    pub overwrite: bool,
    pub metadata_json: bool,
    pub demosaic: Demosaic,
    pub color_matrix: bool,
    pub jpeg_quality: u8,
    pub dng_compression: DngCompressionChoice,
    pub zstd_level: i32,
}

/// Configuration loading or resolution failure.
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("reading configuration {path} failed: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("configuration {path} must use a .toml or .json extension")]
    Unsupported { path: PathBuf },
    #[error("parsing configuration {path} failed: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    #[error("at least one input file or directory is required")]
    MissingInput,
    #[error("jobs must be greater than zero")]
    ZeroJobs,
    #[error("JPEG quality must be within 1..=100")]
    InvalidJpegQuality,
    #[error("unknown demosaic algorithm {0:?}")]
    UnknownDemosaic(String),
}

/// Loads a strict TOML or JSON configuration.
pub fn load_config(path: &Path) -> Result<FileConfigView, ConfigError> {
    let bytes = fs::read(path).map_err(|source| ConfigError::Read {
        path: path.to_owned(),
        source,
    })?;
    let extension = path.extension().and_then(|value| value.to_str());
    let config = match extension {
        Some("toml") => {
            let text = std::str::from_utf8(&bytes).map_err(|source| ConfigError::Parse {
                path: path.to_owned(),
                source: Box::new(source),
            })?;
            toml::from_str::<FileConfig>(text).map_err(|source| ConfigError::Parse {
                path: path.to_owned(),
                source: Box::new(source),
            })?
        }
        Some("json") => {
            serde_json::from_slice::<FileConfig>(&bytes).map_err(|source| ConfigError::Parse {
                path: path.to_owned(),
                source: Box::new(source),
            })?
        }
        _ => {
            return Err(ConfigError::Unsupported {
                path: path.to_owned(),
            });
        }
    };
    Ok(FileConfigView(config))
}

/// Parsed file configuration used by the resolver.
pub struct FileConfigView(FileConfig);

/// Resolves file configuration and explicit CLI overrides.
pub fn resolve_config(args: &CliArgs) -> Result<ResolvedConfig, ConfigError> {
    let config_path = args.config.as_deref();
    let base = config_path
        .and_then(Path::parent)
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let file = config_path
        .map(load_config)
        .transpose()?
        .map_or_else(FileConfig::default, |config| config.0);

    let inputs = if args.inputs.is_empty() {
        file.inputs.unwrap_or_default()
    } else {
        args.inputs.clone()
    };
    if inputs.is_empty() {
        return Err(ConfigError::MissingInput);
    }
    let inputs = inputs
        .into_iter()
        .map(|path| resolve_path(base, path, !args.inputs.is_empty()))
        .collect();
    let output = args.output.clone().or(file.output).map_or_else(
        || PathBuf::from("converted"),
        |path| resolve_path(base, path, args.output.is_some()),
    );
    let jobs = args.jobs.or(file.jobs).unwrap_or(1);
    if jobs == 0 {
        return Err(ConfigError::ZeroJobs);
    }
    let jpeg_quality = args.jpeg_quality.or(file.jpeg_quality).unwrap_or(90);
    if !(1..=100).contains(&jpeg_quality) {
        return Err(ConfigError::InvalidJpegQuality);
    }
    let demosaic_name = args.demosaic.as_deref().or(file.demosaic.as_deref());
    let demosaic = demosaic_name.map_or_else(|| Ok(Demosaic::default()), parse_demosaic)?;
    Ok(ResolvedConfig {
        inputs,
        output,
        format: args.format.or(file.format).unwrap_or_default(),
        jobs,
        recursive: merge_bool(args.recursive, file.recursive),
        overwrite: merge_bool(args.overwrite, file.overwrite),
        metadata_json: merge_bool(args.metadata_json, file.metadata_json),
        demosaic,
        color_matrix: merge_bool(args.color_matrix, file.color_matrix),
        jpeg_quality,
        dng_compression: args
            .dng_compression
            .or(file.dng_compression)
            .unwrap_or_default(),
        zstd_level: args.zstd_level.or(file.zstd_level).unwrap_or(3),
    })
}

fn resolve_path(base: &Path, path: PathBuf, cli_path: bool) -> PathBuf {
    if cli_path || path.is_absolute() {
        path
    } else {
        base.join(path)
    }
}

const fn merge_bool(cli_true: bool, configured: Option<bool>) -> bool {
    if cli_true {
        true
    } else {
        match configured {
            Some(value) => value,
            None => false,
        }
    }
}

fn parse_demosaic(name: &str) -> Result<Demosaic, ConfigError> {
    match name.to_ascii_lowercase().as_str() {
        "aahd" => Ok(Demosaic::Aahd),
        "ahd" => Ok(Demosaic::Ahd),
        "amaze" => Ok(Demosaic::Amaze),
        "dcb" => Ok(Demosaic::dcb()),
        "dht" => Ok(Demosaic::Dht),
        "linear" => Ok(Demosaic::Linear),
        "lmmse" => Ok(Demosaic::lmmse()),
        "ppg" => Ok(Demosaic::Ppg),
        "rcd" => Ok(Demosaic::Rcd),
        "vng" => Ok(Demosaic::Vng),
        "vng4" => Ok(Demosaic::Vng4),
        _ => Err(ConfigError::UnknownDemosaic(name.to_owned())),
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use clap::Parser;
    use fornax::postprocessor::Demosaic;
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn defaults_match_the_converter_contract() {
        let args = CliArgs::try_parse_from(["fornax", "input.dng"]).unwrap();
        let config = resolve_config(&args).unwrap();

        assert_eq!(config.output, PathBuf::from("converted"));
        assert_eq!(config.format, OutputFormat::Dng);
        assert_eq!(config.jobs, 1);
        assert!(config.recursive);
        assert!(!config.overwrite);
        assert!(!config.metadata_json);
        assert_eq!(config.demosaic, Demosaic::default());
        assert!(config.color_matrix);
        assert_eq!(config.jpeg_quality, 90);
        assert_eq!(config.dng_compression, DngCompressionChoice::LosslessJpeg);
    }

    #[test]
    fn config_paths_resolve_from_the_config_directory_and_cli_overrides() {
        let directory = tempdir().unwrap();
        let config_path = directory.path().join("fornax.toml");
        let image = directory.path().join("input.dng");
        std::fs::write(&image, []).unwrap();
        let mut file = std::fs::File::create(&config_path).unwrap();
        writeln!(
            file,
            "inputs = [\"input.dng\"]\noutput = \"out\"\nformat = \"jpeg\"\njobs = 2\ndemosaic = \"ppg\""
        )
        .unwrap();

        let args = CliArgs::try_parse_from([
            "fornax",
            "--config",
            config_path.to_str().unwrap(),
            "--format",
            "tiff",
            "--jobs",
            "1",
        ])
        .unwrap();
        let config = resolve_config(&args).unwrap();

        assert_eq!(config.inputs, vec![image]);
        assert_eq!(config.output, directory.path().join("out"));
        assert_eq!(config.format, OutputFormat::Tiff);
        assert_eq!(config.jobs, 1);
        assert_eq!(config.demosaic, Demosaic::Ppg);
    }

    #[test]
    fn json_config_and_unknown_fields_are_supported_and_rejected() {
        let directory = tempdir().unwrap();
        let valid = directory.path().join("valid.json");
        std::fs::write(&valid, r#"{"inputs":["input.dng"],"format":"jpeg"}"#).unwrap();
        assert!(load_config(&valid).is_ok());

        let invalid = directory.path().join("invalid.json");
        std::fs::write(&invalid, r#"{"input":["typo.dng"]}"#).unwrap();
        assert!(matches!(
            load_config(&invalid),
            Err(ConfigError::Parse { .. })
        ));
    }
}
