use std::marker::PhantomData;
use std::path::{self, Path, PathBuf};
use std::{fmt, fs};

use fornax_dng::Directory;
use fornax_dng::metadata::{IDecoderDng, directory_to_json_value};
use fornax_dng::writer::DngWriter;
use fornax_raw::error::{DecodeError, PostProcessError, PreprocessError};
use fornax_raw::protocol::{IDecoder, IPostProcessor, IPreprocessor};
use fornax_raw::raw_pipeline::{OutputColorSpace, PostProcessor, Preprocessor};
use fornax_raw::{Decoder, RawFormat, RgbImage};
use image::codecs::jpeg::JpegEncoder;
use image::{ExtendedColorType, ImageEncoder};
use num_traits::ToPrimitive;
use rayon::prelude::*;
use thiserror::Error;

use crate::config::{CliArgs, OutputFormat, ResolvedConfig, resolve_config};
use crate::discover::{DiscoveryError, InputJob, plan_inputs};

/// Outcome of one input job.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConversionOutcome {
    Converted { input: PathBuf, output: PathBuf },
    SkippedExisting { input: PathBuf, output: PathBuf },
    SkippedDngToDng { input: PathBuf, output: PathBuf },
    MetadataOnly { input: PathBuf, output: PathBuf },
    Failed { input: PathBuf, error: String },
}

impl fmt::Display for ConversionOutcome {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Converted { input, output } => {
                write!(
                    formatter,
                    "converted {} -> {}",
                    path::absolute(input).unwrap().display(),
                    path::absolute(output).unwrap().display(),
                )
            }
            Self::SkippedExisting { input, output } => write!(
                formatter,
                "skipped existing {} -> {}",
                path::absolute(input).unwrap().display(),
                path::absolute(output).unwrap().display(),
            ),
            Self::SkippedDngToDng { input, output } => write!(
                formatter,
                "skipped DNG-to-DNG {} -> {}",
                path::absolute(input).unwrap().display(),
                path::absolute(output).unwrap().display(),
            ),
            Self::MetadataOnly { input, output } => {
                write!(
                    formatter,
                    "wrote metadata for {} -> {}",
                    path::absolute(input).unwrap().display(),
                    path::absolute(output).unwrap().display(),
                )
            }
            Self::Failed { input, error } => {
                write!(formatter, "failed {}: {error}", input.display())
            }
        }
    }
}

/// Top-level CLI failure before per-file outcomes are available.
#[derive(Debug, Error)]
pub enum CliError {
    #[error(transparent)]
    Config(#[from] crate::ConfigError),
    #[error(transparent)]
    Discovery(#[from] DiscoveryError),
    #[error("creating worker pool failed: {0}")]
    WorkerPool(String),
}

#[derive(Debug, Error)]
enum ConvertError {
    #[error(transparent)]
    Decode(#[from] DecodeError),
    #[error(transparent)]
    PostProcess(#[from] PostProcessError),
    #[error(transparent)]
    Preprocess(#[from] PreprocessError),
    #[error(transparent)]
    DngWrite(#[from] fornax_dng::writer::DngWriteError),
    #[error("creating output directory {path} failed: {source}")]
    OutputDirectory {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("writing output {path} failed: {source}")]
    Output {
        path: PathBuf,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}

/// Executes a CLI invocation.
pub fn run(args: &CliArgs) -> Result<Vec<ConversionOutcome>, CliError> {
    let config = resolve_config(args)?;
    let jobs = plan_inputs(&config)?;
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(config.jobs)
        .build()
        .map_err(|error| CliError::WorkerPool(error.to_string()))?;
    Ok(pool.install(|| {
        jobs.par_iter()
            .map(|job| convert_job(job, &config))
            .collect()
    }))
}

fn convert_job(job: &InputJob, config: &ResolvedConfig) -> ConversionOutcome {
    match convert_job_inner(job, config) {
        Ok(outcome) => outcome,
        Err(error) => ConversionOutcome::Failed {
            input: job.input.clone(),
            error: error.to_string(),
        },
    }
}

fn convert_job_inner(
    job: &InputJob,
    config: &ResolvedConfig,
) -> Result<ConversionOutcome, ConvertError> {
    if job.output.exists() && !config.overwrite {
        return Ok(ConversionOutcome::SkippedExisting {
            input: job.input.clone(),
            output: job.output.clone(),
        });
    }
    match config.format {
        OutputFormat::Dng if job.format == RawFormat::Dng => {
            if config.metadata_json {
                let decoder = Decoder::decode(&job.input)?;
                create_parent(&job.output)?;
                write_metadata_json(&job.output, &decoder.dng_metadata()?)?;
                return Ok(ConversionOutcome::MetadataOnly {
                    input: job.input.clone(),
                    output: job.output.with_extension("json"),
                });
            }
            Ok(ConversionOutcome::SkippedDngToDng {
                input: job.input.clone(),
                output: job.output.clone(),
            })
        }
        OutputFormat::Dng => convert_raw_dng(job, config),
        OutputFormat::Tiff | OutputFormat::Jpeg => {
            render_with(Decoder::decode(&job.input)?, job, config)
        }
    }
}

fn convert_raw_dng(
    job: &InputJob,
    config: &ResolvedConfig,
) -> Result<ConversionOutcome, ConvertError> {
    create_parent(&job.output)?;
    let decoder = Decoder::decode(&job.input)?;
    let mosaic = decoder.raw_mosaic()?;
    let metadata = decoder.dng_metadata()?;
    DngWriter::new(config.dng_compression.writer())
        .with_zstd_level(config.zstd_level)
        .write(&job.output, &mosaic, &metadata)?;
    if config.metadata_json {
        write_metadata_json(&job.output, &metadata)?;
    }
    Ok(ConversionOutcome::Converted {
        input: job.input.clone(),
        output: job.output.clone(),
    })
}

fn render_with<D>(
    decoder: D,
    job: &InputJob,
    config: &ResolvedConfig,
) -> Result<ConversionOutcome, ConvertError>
where
    D: IDecoderDng,
{
    let cropped = decoder.cropped_mosaic()?;
    let metadata = decoder.process_metadata()?;
    let normalized = Preprocessor.process(cropped, &metadata.levels)?;
    let processor = PostProcessor {
        demosaic: config.demosaic,
        output_color: if config.color_matrix {
            OutputColorSpace::Srgb
        } else {
            OutputColorSpace::Raw
        },
        _mark: PhantomData,
    };
    let rgb = processor.process(normalized.image(), &metadata)?;
    create_parent(&job.output)?;
    match config.format {
        OutputFormat::Tiff => write_tiff(&rgb, &job.output)?,
        OutputFormat::Jpeg => write_jpeg(&rgb, &job.output, config.jpeg_quality)?,
        OutputFormat::Dng => unreachable!("raw DNG conversion has a separate path"),
    }
    if config.metadata_json {
        write_metadata_json(&job.output, &decoder.dng_metadata()?)?;
    }
    Ok(ConversionOutcome::Converted {
        input: job.input.clone(),
        output: job.output.clone(),
    })
}

fn write_tiff(image: &RgbImage<f32>, path: &Path) -> Result<(), ConvertError> {
    let (width, height) = image.dimensions();
    let samples = image
        .as_raw()
        .iter()
        .map(|value| {
            (value.clamp(0.0, 1.0) * f32::from(u16::MAX))
                .round()
                .to_u16()
                .unwrap_or_default()
        })
        .collect();
    let tiff = image::ImageBuffer::<image::Rgb<u16>, Vec<u16>>::from_raw(width, height, samples)
        .ok_or_else(|| ConvertError::Output {
            path: path.to_owned(),
            source: "RGB dimensions are inconsistent".into(),
        })?;
    tiff.save(path).map_err(|source| ConvertError::Output {
        path: path.to_owned(),
        source: Box::new(source),
    })
}

fn write_jpeg(image: &RgbImage<f32>, path: &Path, quality: u8) -> Result<(), ConvertError> {
    let (width, height) = image.dimensions();
    let pixels: Vec<u8> = image
        .as_raw()
        .iter()
        .map(|value| {
            let encoded = srgb_encode(value.clamp(0.0, 1.0));
            (encoded * 255.0).round() as u8
        })
        .collect();
    let file = fs::File::create(path).map_err(|source| ConvertError::Output {
        path: path.to_owned(),
        source: Box::new(source),
    })?;
    JpegEncoder::new_with_quality(file, quality)
        .write_image(&pixels, width, height, ExtendedColorType::Rgb8)
        .map_err(|source| ConvertError::Output {
            path: path.to_owned(),
            source: Box::new(source),
        })
}

fn srgb_encode(value: f32) -> f32 {
    if value <= 0.003_130_8 {
        12.92 * value
    } else {
        1.055_f32.mul_add(value.powf(1.0 / 2.4), -0.055)
    }
}

fn create_parent(path: &Path) -> Result<(), ConvertError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| ConvertError::OutputDirectory {
            path: parent.to_owned(),
            source,
        })?;
    }
    Ok(())
}

fn write_metadata_json(output: &Path, metadata: &Directory) -> Result<(), ConvertError> {
    let path = output.with_extension("json");
    let bytes =
        serde_json::to_vec_pretty(&directory_to_json_value(metadata)).map_err(|source| {
            ConvertError::Output {
                path: path.clone(),
                source: Box::new(source),
            }
        })?;
    fs::write(&path, bytes).map_err(|source| ConvertError::Output {
        path: path.clone(),
        source: Box::new(source),
    })
}
