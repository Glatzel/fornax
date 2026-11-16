#[allow(dead_code, reason = "shared with the Criterion decoder benchmark")]
mod decoder_inputs;
mod flashraw_wrapper;

use std::collections::HashMap;
use std::env;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use csv::{ReaderBuilder, WriterBuilder};
use flashraw_wrapper::FlashRawDecoder;
use fornax_raw_core::metadata::Levels;
use fornax_raw_core::protocol::IDecoder;
use fornax_raw_core::{CroppedMosaic, RawMosaic};
use fornax_raw_decoder::Decoder;
use fornax_raw_pipeline::Preprocessor;
use sha2::{Digest, Sha256};

const DEFAULT_BASELINE: &str = "crates/decoder-tracker/data/baseline.csv";
const DEFAULT_OUTPUT: &str = "target/flashraw-031/probe.csv";

#[derive(Clone, Debug)]
struct SourceMetadata {
    format: String,
    compression: String,
    bits_per_sample: String,
}

#[derive(Default)]
struct Usage {
    minor_faults: i64,
    major_faults: i64,
}

struct StageTiming {
    duration: Duration,
    usage: Usage,
}

#[derive(Clone, Copy)]
enum ProbeDecoder {
    Fornax,
    FlashRaw,
}

impl ProbeDecoder {
    fn name(self) -> &'static str {
        match self {
            Self::Fornax => "fornax",
            Self::FlashRaw => "flashraw",
        }
    }
}

fn main() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let baseline = env::var_os("FORNAX_PROBE_BASELINE").map_or_else(
        || repo_root.join(DEFAULT_BASELINE),
        |path| resolve_repo_path(&repo_root, PathBuf::from(path)),
    );
    let output = env::var_os("FORNAX_PROBE_OUT").map_or_else(
        || repo_root.join(DEFAULT_OUTPUT),
        |path| resolve_repo_path(&repo_root, PathBuf::from(path)),
    );
    let warmups = env::var("FORNAX_PROBE_WARMUPS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(2_u32);
    let rounds = env::var("FORNAX_PROBE_ROUNDS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(7_u32);
    let batch = env::var("FORNAX_PROBE_BATCH").unwrap_or_else(|_| "1".into());
    let decoders = env::var("FORNAX_PROBE_DECODERS")
        .unwrap_or_else(|_| "fornax,flashraw".into())
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(parse_decoder)
        .collect::<Vec<_>>();
    eprintln!(
        "probe configuration: rayon_threads={} flashraw_threads={:?}",
        rayon::current_num_threads(),
        env::var("FORNAX_FLASHRAW_THREADS").ok()
    );

    let metadata = read_metadata(&baseline).unwrap_or_else(|error| {
        panic!(
            "reading benchmark metadata {} failed: {error}",
            baseline.display()
        )
    });
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).unwrap_or_else(|error| {
            panic!(
                "creating probe output directory {} failed: {error}",
                parent.display()
            )
        });
    }
    let file = File::create(&output).unwrap_or_else(|error| {
        panic!("creating probe output {} failed: {error}", output.display())
    });
    let mut writer = WriterBuilder::new().from_writer(file);
    writer
        .write_record([
            "batch",
            "decoder",
            "format",
            "compression",
            "path",
            "stage",
            "round",
            "duration_ns",
            "minor_faults",
            "major_faults",
            "raw_sha256",
            "cropped_sha256",
            "width",
            "height",
            "bits_per_sample",
        ])
        .expect("writing probe header");

    for (input, path) in decoder_inputs::benchmark_inputs() {
        let source = metadata.get(&input.path).unwrap_or_else(|| {
            panic!("benchmark input {} is absent from the baseline", input.path)
        });
        let fornax = Decoder::decode(&path).unwrap_or_else(|error| {
            panic!("Fornax metadata decode failed for {}: {error}", input.path)
        });
        let process = fornax.process_metadata().unwrap_or_else(|error| {
            panic!("Fornax process metadata failed for {}: {error}", input.path)
        });
        for decoder in &decoders {
            match decoder {
                ProbeDecoder::Fornax => probe::<Decoder>(
                    *decoder,
                    &input.path,
                    &path,
                    source,
                    &process.levels,
                    &batch,
                    warmups,
                    rounds,
                    &mut writer,
                ),
                ProbeDecoder::FlashRaw => probe::<FlashRawDecoder>(
                    *decoder,
                    &input.path,
                    &path,
                    source,
                    &process.levels,
                    &batch,
                    warmups,
                    rounds,
                    &mut writer,
                ),
            }
        }
    }
    writer.flush().expect("flushing probe output");
    eprintln!("wrote {}", output.display());
}

fn parse_decoder(value: &str) -> ProbeDecoder {
    match value {
        "fornax" => ProbeDecoder::Fornax,
        "flashraw" => ProbeDecoder::FlashRaw,
        _ => panic!("unknown probe decoder {value:?}; expected fornax or flashraw"),
    }
}

fn resolve_repo_path(repo_root: &Path, path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        repo_root.join(path)
    }
}

fn read_metadata(path: &Path) -> Result<HashMap<String, SourceMetadata>, String> {
    let mut reader = ReaderBuilder::new()
        .from_path(path)
        .map_err(|error| error.to_string())?;
    let mut metadata = HashMap::new();
    for record in reader.records() {
        let record = record.map_err(|error| error.to_string())?;
        let source = SourceMetadata {
            format: record.get(2).unwrap_or_default().to_owned(),
            compression: record.get(14).unwrap_or_default().to_owned(),
            bits_per_sample: record.get(17).unwrap_or_default().to_owned(),
        };
        metadata.insert(record.get(3).unwrap_or_default().to_owned(), source);
    }
    Ok(metadata)
}

#[allow(clippy::too_many_arguments)]
fn probe<D: IDecoder>(
    decoder: ProbeDecoder,
    source_path: &str,
    path: &Path,
    source: &SourceMetadata,
    levels: &Levels,
    batch: &str,
    warmups: u32,
    rounds: u32,
    writer: &mut csv::Writer<File>,
) {
    for _ in 0..warmups {
        decode::<D>(path);
        pipeline::<D>(path, levels);
    }

    let (decoded, _) = timed(|| decode::<D>(path));
    let raw = decoded.raw_mosaic().unwrap_or_else(|error| {
        panic!(
            "{} raw_mosaic failed for {source_path}: {error}",
            decoder.name()
        )
    });
    let (width, height) = raw.dimensions();
    let raw_sha256 = hash_raw(&raw);
    let (cropped, _) = timed(|| {
        decoded.cropped_mosaic::<f32>().unwrap_or_else(|error| {
            panic!(
                "{} cropped_mosaic failed for {source_path}: {error}",
                decoder.name()
            )
        })
    });
    let cropped_sha256 = hash_cropped(&cropped);

    for round in 0..rounds {
        record_stage(
            writer,
            batch,
            decoder,
            source,
            source_path,
            "decode",
            round,
            timed(|| decode::<D>(path)),
            raw_sha256.as_str(),
            cropped_sha256.as_str(),
            width,
            height,
        );
        let preprocess_input = cropped.clone();
        record_stage(
            writer,
            batch,
            decoder,
            source,
            source_path,
            "crop_cast",
            round,
            timed(|| {
                decoded.cropped_mosaic::<f32>().unwrap_or_else(|error| {
                    panic!(
                        "{} cropped_mosaic failed for {source_path}: {error}",
                        decoder.name()
                    )
                })
            }),
            raw_sha256.as_str(),
            cropped_sha256.as_str(),
            width,
            height,
        );
        record_stage(
            writer,
            batch,
            decoder,
            source,
            source_path,
            "preprocess",
            round,
            timed(|| {
                preprocess_input
                    .process(&Preprocessor, levels)
                    .unwrap_or_else(|error| panic!("preprocess failed for {source_path}: {error}"))
            }),
            raw_sha256.as_str(),
            cropped_sha256.as_str(),
            width,
            height,
        );
        record_stage(
            writer,
            batch,
            decoder,
            source,
            source_path,
            "pipeline",
            round,
            timed(|| pipeline::<D>(path, levels)),
            raw_sha256.as_str(),
            cropped_sha256.as_str(),
            width,
            height,
        );
    }
}

fn decode<D: IDecoder>(path: &Path) -> D {
    D::decode(path).unwrap_or_else(|error| panic!("decoder failed for {}: {error}", path.display()))
}

fn pipeline<D: IDecoder>(path: &Path, levels: &Levels) {
    let decoded = decode::<D>(path);
    let cropped = decoded
        .cropped_mosaic::<f32>()
        .unwrap_or_else(|error| panic!("cropped_mosaic failed for {}: {error}", path.display()));
    cropped
        .process(&Preprocessor, levels)
        .unwrap_or_else(|error| panic!("preprocess failed for {}: {error}", path.display()));
}

fn timed<T>(operation: impl FnOnce() -> T) -> (T, StageTiming) {
    let before = usage();
    let start = Instant::now();
    let output = operation();
    let duration = start.elapsed();
    let after = usage();
    (
        output,
        StageTiming {
            duration,
            usage: Usage {
                minor_faults: after.minor_faults - before.minor_faults,
                major_faults: after.major_faults - before.major_faults,
            },
        },
    )
}

#[allow(clippy::too_many_arguments)]
fn record_stage<T>(
    writer: &mut csv::Writer<File>,
    batch: &str,
    decoder: ProbeDecoder,
    source: &SourceMetadata,
    path: &str,
    stage: &str,
    round: u32,
    (_, timing): (T, StageTiming),
    raw_sha256: &str,
    cropped_sha256: &str,
    width: u32,
    height: u32,
) {
    writer
        .write_record([
            batch,
            decoder.name(),
            source.format.as_str(),
            source.compression.as_str(),
            path,
            stage,
            &round.to_string(),
            &timing.duration.as_nanos().to_string(),
            &timing.usage.minor_faults.to_string(),
            &timing.usage.major_faults.to_string(),
            raw_sha256,
            cropped_sha256,
            &width.to_string(),
            &height.to_string(),
            source.bits_per_sample.as_str(),
        ])
        .expect("writing probe row");
}

fn usage() -> Usage {
    let mut usage = std::mem::MaybeUninit::<libc::rusage>::uninit();
    let status = unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) };
    assert_eq!(status, 0, "getrusage failed");
    let usage = unsafe { usage.assume_init() };
    Usage {
        minor_faults: usage.ru_minflt,
        major_faults: usage.ru_majflt,
    }
}

fn hash_raw(raw: &RawMosaic) -> String {
    let mut hash = Sha256::new();
    let (width, height) = raw.dimensions();
    hash.update(width.to_le_bytes());
    hash.update(height.to_le_bytes());
    match raw {
        RawMosaic::U8(image) => {
            hash.update([1]);
            hash.update(image.as_raw());
        }
        RawMosaic::U16(image) => {
            hash.update([2]);
            for sample in image.as_raw() {
                hash.update(sample.to_le_bytes());
            }
        }
        RawMosaic::U32(image) => {
            hash.update([3]);
            for sample in image.as_raw() {
                hash.update(sample.to_le_bytes());
            }
        }
        RawMosaic::F32(image) => {
            hash.update([4]);
            for sample in image.as_raw() {
                hash.update(sample.to_bits().to_le_bytes());
            }
        }
    }
    hex(&hash.finalize())
}

fn hash_cropped(image: &CroppedMosaic<f32>) -> String {
    let mut hash = Sha256::new();
    let (width, height) = image.image().dimensions();
    hash.update(width.to_le_bytes());
    hash.update(height.to_le_bytes());
    for sample in image.image().as_raw() {
        hash.update(sample.to_bits().to_le_bytes());
    }
    hex(&hash.finalize())
}

fn hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write;
        write!(output, "{byte:02x}").expect("writing to a string");
    }
    output
}
