#[allow(dead_code, reason = "shared with the Criterion decoder benchmark")]
#[path = "../decoder_inputs.rs"]
mod decoder_inputs;

use std::collections::HashMap;
use std::env;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use csv::{ReaderBuilder, WriterBuilder};
use flashraw::{
    Backend, CorruptionPolicy, DecodeOptions, FlashRaw, OutputFormat, RawFrame, RawPixelData,
    SampleFormat,
};
use fornax_raw_core::RawMosaic;
use fornax_raw_core::error::DecodeError;
use fornax_raw_core::protocol::IDecoder;
use fornax_raw_decoder::Decoder;
use sha2::{Digest, Sha256};

const DEFAULT_BASELINE: &str = "crates/decoder-tracker/data/baseline.csv";
const DEFAULT_OUTPUT: &str = "target/flashraw-031/raw-decode.csv";
const RAW_DOMAIN: &[u8] = b"fornax.raw_mosaic.v1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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

#[derive(Clone, Debug)]
struct SourceMetadata {
    brand: String,
    model: String,
    format: String,
    compression: String,
    mode: String,
    bits_per_sample: String,
    reference_sha256: String,
}

#[derive(Clone, Copy)]
struct ProbeConfig {
    backend: Backend,
    flashraw_threads: Option<usize>,
    threads_label: usize,
}

#[derive(Clone, Copy, Debug, Default)]
struct Usage {
    minor_faults: i64,
    major_faults: i64,
}

struct StageTiming {
    total: Duration,
    open_path: Option<Duration>,
    decode: Duration,
    usage: Usage,
}

enum DecodedRaw {
    Fornax(RawMosaic),
    FlashRaw(Box<RawFrame>),
}

impl DecodedRaw {
    fn dimensions(&self) -> (u32, u32) {
        match self {
            Self::Fornax(raw) => raw.dimensions(),
            Self::FlashRaw(frame) => (frame.width, frame.height),
        }
    }

    fn sample_type(&self) -> &'static str {
        match self {
            Self::Fornax(RawMosaic::U8(_)) => "u8",
            Self::Fornax(RawMosaic::U16(_)) => "u16",
            Self::Fornax(RawMosaic::U32(_)) => "u32",
            Self::Fornax(RawMosaic::F32(_)) => "f32",
            Self::FlashRaw(frame) => match &frame.data {
                RawPixelData::U8(_) => "u8",
                RawPixelData::U16(_) => "u16",
                RawPixelData::F32(_) => "f32",
                _ => "unsupported",
            },
        }
    }
}

enum ProbeFailure {
    Unsupported(String),
    Error(String),
}

impl ProbeFailure {
    fn status(&self) -> &'static str {
        match self {
            Self::Unsupported(_) => "unsupported",
            Self::Error(_) => "error",
        }
    }

    fn message(&self) -> &str {
        match self {
            Self::Unsupported(message) | Self::Error(message) => message,
        }
    }
}

struct TimedRaw {
    raw: DecodedRaw,
    timing: StageTiming,
}

#[derive(Clone)]
struct RoundResult {
    open_ns: Option<u64>,
    decode_ns: u64,
    total_ns: u64,
    minor_faults: i64,
    major_faults: i64,
}

pub(super) fn main() {
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
        .unwrap_or(5_u32);
    let batch = env::var("FORNAX_PROBE_BATCH").unwrap_or_else(|_| "1".into());
    let decoder =
        parse_decoder(&env::var("FORNAX_PROBE_DECODERS").unwrap_or_else(|_| "flashraw".into()));
    let config = probe_config(decoder)
        .unwrap_or_else(|error| panic!("invalid probe configuration: {error}"));
    let backend = match decoder {
        ProbeDecoder::Fornax => "native",
        ProbeDecoder::FlashRaw => backend_name(config.backend),
    };

    eprintln!(
        "probe configuration: decoder={} backend={backend} threads={} flashraw_threads={:?} rayon_threads={}",
        decoder.name(),
        config.threads_label,
        config.flashraw_threads,
        rayon::current_num_threads()
    );

    let metadata = read_metadata(&baseline)
        .unwrap_or_else(|error| panic!("reading benchmark metadata failed: {error}"));
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
            "backend",
            "threads",
            "brand",
            "model",
            "format",
            "compression",
            "mode",
            "sample",
            "stage",
            "round",
            "status",
            "error",
            "open_ms",
            "decode_ms",
            "total_ms",
            "minor_faults",
            "major_faults",
            "raw_sha256",
            "reference_sha256",
            "matches_reference",
            "sample_type",
            "width",
            "height",
            "bits_per_sample",
        ])
        .expect("writing probe header");

    for (input, path) in decoder_inputs::benchmark_inputs() {
        let source = metadata
            .get(&input.path)
            .unwrap_or_else(|| panic!("benchmark input {} has no matrix metadata", input.path));
        probe_sample(
            decoder,
            &path,
            &input.path,
            source,
            &batch,
            backend,
            config,
            warmups,
            rounds,
            &mut writer,
        );
    }
    writer.flush().expect("flushing probe output");
    eprintln!("wrote {}", output.display());
}

fn resolve_repo_path(repo_root: &Path, path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        repo_root.join(path)
    }
}

fn parse_decoder(value: &str) -> ProbeDecoder {
    match value.trim() {
        "fornax" => ProbeDecoder::Fornax,
        "flashraw" => ProbeDecoder::FlashRaw,
        other => panic!("unknown decoder {other:?}; expected fornax or flashraw"),
    }
}

fn probe_config(decoder: ProbeDecoder) -> Result<ProbeConfig, String> {
    let backend = if decoder == ProbeDecoder::FlashRaw {
        decode_backend()?
    } else {
        Backend::Cpu
    };
    let flashraw_threads = if decoder == ProbeDecoder::FlashRaw {
        decode_threads()?
    } else {
        None
    };
    let threads_label = match decoder {
        ProbeDecoder::Fornax => rayon::current_num_threads(),
        ProbeDecoder::FlashRaw => flashraw_threads.unwrap_or_else(rayon::current_num_threads),
    };
    Ok(ProbeConfig {
        backend,
        flashraw_threads,
        threads_label,
    })
}

fn decode_backend() -> Result<Backend, String> {
    let Some(value) = env::var_os("FORNAX_FLASHRAW_BACKEND") else {
        return Ok(Backend::Cpu);
    };
    let value = value
        .to_str()
        .ok_or_else(|| "FORNAX_FLASHRAW_BACKEND is not UTF-8".to_owned())?;
    let backend = Backend::parse(value).ok_or_else(|| {
        format!("invalid FlashRaw backend {value:?}; expected cpu, simd, or auto")
    })?;
    match backend {
        Backend::Cpu | Backend::CpuSimd | Backend::Auto => Ok(backend),
        _ => Err(format!(
            "unsupported FlashRaw backend {value:?}; expected cpu, simd, or auto"
        )),
    }
}

fn decode_threads() -> Result<Option<usize>, String> {
    let Some(value) = env::var_os("FORNAX_FLASHRAW_THREADS") else {
        return Ok(Some(1));
    };
    let value = value
        .to_str()
        .ok_or_else(|| "FORNAX_FLASHRAW_THREADS is not UTF-8".to_owned())?;
    if value == "all" || value == "0" {
        return Ok(None);
    }
    let threads = value
        .parse::<usize>()
        .map_err(|error| format!("invalid FlashRaw thread count {value:?}: {error}"))?;
    Ok(Some(threads))
}

fn backend_name(backend: Backend) -> &'static str {
    match backend {
        Backend::Cpu => "cpu",
        Backend::CpuSimd => "simd",
        Backend::Auto => "auto",
        _ => "other",
    }
}

fn read_metadata(baseline_path: &Path) -> Result<HashMap<String, SourceMetadata>, String> {
    let mut reader = ReaderBuilder::new()
        .from_path(baseline_path)
        .map_err(|error| error.to_string())?;
    let headers = reader.headers().map_err(|error| error.to_string())?.clone();
    let index = |name: &str| {
        headers
            .iter()
            .position(|header| header == name)
            .ok_or_else(|| format!("baseline is missing {name}"))
    };
    let path_index = index("path")?;
    let camera_index = index("camera")?;
    let format_index = index("format")?;
    let compression_index = index("compression_name")?;
    let bits_index = index("bits_per_sample")?;
    let hash_index = index("raw_sha256")?;
    let mut metadata = HashMap::new();
    for record in reader.records() {
        let record = record.map_err(|error| error.to_string())?;
        let path = record.get(path_index).unwrap_or_default();
        let compression = record.get(compression_index).unwrap_or_default().to_owned();
        metadata.insert(
            path.to_owned(),
            SourceMetadata {
                brand: path.split('/').next().unwrap_or_default().to_owned(),
                model: record.get(camera_index).unwrap_or_default().to_owned(),
                format: record.get(format_index).unwrap_or_default().to_owned(),
                mode: compression.clone(),
                compression,
                bits_per_sample: record.get(bits_index).unwrap_or_default().to_owned(),
                reference_sha256: record.get(hash_index).unwrap_or_default().to_owned(),
            },
        );
    }
    Ok(metadata)
}

fn probe_sample(
    decoder: ProbeDecoder,
    path: &Path,
    sample: &str,
    source: &SourceMetadata,
    batch: &str,
    backend: &str,
    config: ProbeConfig,
    warmups: u32,
    rounds: u32,
    writer: &mut csv::Writer<File>,
) {
    let verification = match measure_raw_decode(decoder, path, config) {
        Ok(verification) => verification,
        Err(error) => {
            write_failure(
                writer, decoder, sample, source, batch, backend, config, &error, None,
            );
            return;
        }
    };
    let (raw_sha256, sample_type, width, height) = match hash_raw(&verification.raw) {
        Ok(result) => result,
        Err(error) => {
            write_failure(
                writer,
                decoder,
                sample,
                source,
                batch,
                backend,
                config,
                &ProbeFailure::Error(error),
                None,
            );
            return;
        }
    };
    if raw_sha256 != source.reference_sha256 {
        write_failure(
            writer,
            decoder,
            sample,
            source,
            batch,
            backend,
            config,
            &ProbeFailure::Error(format!(
                "raw hash mismatch before timing: decoded {raw_sha256}, expected {}",
                source.reference_sha256
            )),
            None,
        );
        return;
    }
    drop(verification);

    for _ in 0..warmups {
        match measure_raw_decode(decoder, path, config) {
            Ok(timed) => drop(timed.raw),
            Err(error) => {
                write_failure(
                    writer, decoder, sample, source, batch, backend, config, &error, None,
                );
                return;
            }
        }
    }

    let mut results = Vec::with_capacity(rounds as usize);
    for round in 0..rounds {
        match measure_raw_decode(decoder, path, config) {
            Ok(timed) => {
                results.push(RoundResult {
                    open_ns: timed.timing.open_path.map(duration_ns),
                    decode_ns: duration_ns(timed.timing.decode),
                    total_ns: duration_ns(timed.timing.total),
                    minor_faults: timed.timing.usage.minor_faults,
                    major_faults: timed.timing.usage.major_faults,
                });
                drop(timed.raw);
            }
            Err(error) => {
                write_failure(
                    writer,
                    decoder,
                    sample,
                    source,
                    batch,
                    backend,
                    config,
                    &error,
                    Some(round),
                );
                return;
            }
        }
    }

    for (round, result) in results.into_iter().enumerate() {
        writer
            .write_record([
                batch,
                decoder.name(),
                backend,
                &config.threads_label.to_string(),
                source.brand.as_str(),
                source.model.as_str(),
                source.format.as_str(),
                source.compression.as_str(),
                source.mode.as_str(),
                sample,
                "raw_mosaic",
                &round.to_string(),
                "ok",
                "",
                &optional_ms(result.open_ns),
                &nanoseconds_to_ms(result.decode_ns),
                &nanoseconds_to_ms(result.total_ns),
                &result.minor_faults.to_string(),
                &result.major_faults.to_string(),
                raw_sha256.as_str(),
                source.reference_sha256.as_str(),
                "1",
                sample_type,
                &width.to_string(),
                &height.to_string(),
                source.bits_per_sample.as_str(),
            ])
            .expect("writing probe row");
    }
}

fn measure_raw_decode(
    decoder: ProbeDecoder,
    path: &Path,
    config: ProbeConfig,
) -> Result<TimedRaw, ProbeFailure> {
    match decoder {
        ProbeDecoder::Fornax => measure_fornax(path),
        ProbeDecoder::FlashRaw => measure_flashraw(path, config),
    }
}

fn measure_fornax(path: &Path) -> Result<TimedRaw, ProbeFailure> {
    let before = usage();
    let total_start = Instant::now();
    let decoder = Decoder::decode(path).map_err(fornax_failure)?;
    let raw = decoder.raw_mosaic().map_err(fornax_failure)?;
    let decode = total_start.elapsed();
    let after = usage();
    Ok(TimedRaw {
        raw: DecodedRaw::Fornax(raw),
        timing: StageTiming {
            total: decode,
            open_path: None,
            decode,
            usage: subtract_usage(after, before),
        },
    })
}

fn measure_flashraw(path: &Path, config: ProbeConfig) -> Result<TimedRaw, ProbeFailure> {
    let before = usage();
    let total_start = Instant::now();
    let open_start = Instant::now();
    let raw = FlashRaw::open_path(path).map_err(|error| {
        ProbeFailure::Error(format!(
            "FlashRaw open_path failed for {}: {error}",
            path.display()
        ))
    })?;
    let open_path = open_start.elapsed();

    let decode_start = Instant::now();
    let output = match raw.frame_info().sample_format {
        SampleFormat::U16 => OutputFormat::RawU16,
        SampleFormat::F32 => OutputFormat::RawF32,
        sample_format => {
            return Err(ProbeFailure::Unsupported(format!(
                "FlashRaw sample format {sample_format:?} is unsupported"
            )));
        }
    };
    let frame = raw
        .decode(DecodeOptions {
            backend: config.backend,
            output,
            threads: config.flashraw_threads,
            crop: false,
            apply_opcodes: false,
            corruption_policy: CorruptionPolicy::Strict,
            ..DecodeOptions::default()
        })
        .map_err(|error| {
            ProbeFailure::Error(format!(
                "FlashRaw decode failed for {}: {error}",
                path.display()
            ))
        })?;
    let decode = decode_start.elapsed();
    let total = total_start.elapsed();
    let after = usage();
    Ok(TimedRaw {
        raw: DecodedRaw::FlashRaw(Box::new(frame)),
        timing: StageTiming {
            total,
            open_path: Some(open_path),
            decode,
            usage: subtract_usage(after, before),
        },
    })
}

fn fornax_failure(error: DecodeError) -> ProbeFailure {
    match error {
        DecodeError::Unsupported(_) => ProbeFailure::Unsupported(error.to_string()),
        _ => ProbeFailure::Error(error.to_string()),
    }
}

fn subtract_usage(after: Usage, before: Usage) -> Usage {
    Usage {
        minor_faults: after.minor_faults - before.minor_faults,
        major_faults: after.major_faults - before.major_faults,
    }
}

fn write_failure(
    writer: &mut csv::Writer<File>,
    decoder: ProbeDecoder,
    sample: &str,
    source: &SourceMetadata,
    batch: &str,
    backend: &str,
    config: ProbeConfig,
    failure: &ProbeFailure,
    round: Option<u32>,
) {
    writer
        .write_record([
            batch,
            decoder.name(),
            backend,
            &config.threads_label.to_string(),
            source.brand.as_str(),
            source.model.as_str(),
            source.format.as_str(),
            source.compression.as_str(),
            source.mode.as_str(),
            sample,
            "raw_mosaic",
            &round.map_or_else(String::new, |round| round.to_string()),
            failure.status(),
            failure.message(),
            "",
            "",
            "",
            "",
            "",
            "",
            source.reference_sha256.as_str(),
            "",
            "",
            "",
            "",
            source.bits_per_sample.as_str(),
        ])
        .expect("writing probe failure row");
}

fn duration_ns(duration: Duration) -> u64 {
    u64::try_from(duration.as_nanos()).expect("duration fits u64 nanoseconds")
}

fn nanoseconds_to_ms(nanoseconds: u64) -> String {
    format!("{:.6}", nanoseconds as f64 / 1_000_000.0)
}

fn optional_ms(nanoseconds: Option<u64>) -> String {
    nanoseconds.map_or_else(String::new, nanoseconds_to_ms)
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

fn hash_raw(raw: &DecodedRaw) -> Result<(String, &'static str, u32, u32), String> {
    let (width, height) = raw.dimensions();
    let mut hash = Sha256::new();
    update_bytes(&mut hash, RAW_DOMAIN);
    match raw {
        DecodedRaw::Fornax(RawMosaic::U8(image)) => {
            update_samples(
                &mut hash,
                "u8",
                width,
                height,
                image.as_raw().iter().copied(),
            );
        }
        DecodedRaw::Fornax(RawMosaic::U16(image)) => {
            update_samples(
                &mut hash,
                "u16",
                width,
                height,
                image
                    .as_raw()
                    .iter()
                    .flat_map(|sample| sample.to_le_bytes()),
            );
        }
        DecodedRaw::Fornax(RawMosaic::U32(image)) => {
            update_samples(
                &mut hash,
                "u32",
                width,
                height,
                image
                    .as_raw()
                    .iter()
                    .flat_map(|sample| sample.to_le_bytes()),
            );
        }
        DecodedRaw::Fornax(RawMosaic::F32(image)) => {
            update_samples(
                &mut hash,
                "f32",
                width,
                height,
                image
                    .as_raw()
                    .iter()
                    .flat_map(|sample| sample.to_bits().to_le_bytes()),
            );
        }
        DecodedRaw::FlashRaw(frame) => {
            if frame.cpp != 1 {
                return Err(format!(
                    "FlashRaw returned {}-sample pixels, expected one",
                    frame.cpp
                ));
            }
            match &frame.data {
                RawPixelData::U16(samples) => {
                    update_samples(
                        &mut hash,
                        "u16",
                        width,
                        height,
                        samples.iter().flat_map(|sample| sample.to_le_bytes()),
                    );
                }
                RawPixelData::F32(samples) => {
                    update_samples(
                        &mut hash,
                        "f32",
                        width,
                        height,
                        samples
                            .iter()
                            .flat_map(|sample| sample.to_bits().to_le_bytes()),
                    );
                }
                other => {
                    return Err(format!(
                        "unsupported FlashRaw raw storage {:?}",
                        std::mem::discriminant(other)
                    ));
                }
            }
        }
    }
    Ok((
        hex(hash.finalize().as_slice()),
        raw.sample_type(),
        width,
        height,
    ))
}

fn update_samples(
    hash: &mut Sha256,
    sample_type: &str,
    width: u32,
    height: u32,
    samples: impl IntoIterator<Item = u8>,
) {
    update_bytes(hash, sample_type.as_bytes());
    hash.update(width.to_le_bytes());
    hash.update(height.to_le_bytes());
    for byte in samples {
        hash.update([byte]);
    }
}

fn update_bytes(hash: &mut Sha256, bytes: &[u8]) {
    hash.update((bytes.len() as u64).to_le_bytes());
    hash.update(bytes);
}

fn hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write;
        write!(output, "{byte:02x}").expect("writing to a string");
    }
    output
}
