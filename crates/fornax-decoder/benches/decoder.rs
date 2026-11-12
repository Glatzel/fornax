mod decoder_inputs;
mod decoder_selection;
mod flashraw_wrapper;
mod rawler_wrapper;
mod reference_preflight;

use std::path::{Path, PathBuf};
use std::{env, fs};

use criterion::measurement::WallTime;
use criterion::{BenchmarkGroup, Criterion, black_box, criterion_group, criterion_main};
use decoder_inputs::{
    GoldenInput, ensure_materialized, parse_golden_manifest, resolve_repo_relative,
    select_golden_inputs,
};
use decoder_selection::{DecoderKind, parse_decoder_selection};
use flashraw_wrapper::FlashRawDecoder;
use fornax_decoder::Decoder;
use fornax_dng::metadata::IDecoderDng;
use fornax_raw_core::protocol::IDecoder;
use rawler_wrapper::RawlerDecoder;
use reference_preflight::{ReferencePreflight, preflight_reference};

const DEFAULT_MANIFEST: &str = "crates/decoder-tracker/golden/selected-manifest.csv";
const DEFAULT_CORPUS: &str = "external/raw.pixls.us";

fn selected_decoders() -> Vec<DecoderKind> {
    let value = if env::var("CI").is_ok() {
        Some("fornax".into())
    } else {
        env::var("FORNAX_BENCH_DECODER").ok()
    };
    parse_decoder_selection(value.as_deref())
        .unwrap_or_else(|error| panic!("parsing FORNAX_BENCH_DECODER failed: {error}"))
}

fn benchmark_inputs() -> Vec<(GoldenInput, PathBuf)> {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let manifest = env::var_os("FORNAX_BENCH_MANIFEST")
        .map_or_else(|| PathBuf::from(DEFAULT_MANIFEST), PathBuf::from);
    let corpus = env::var_os("FORNAX_BENCH_CORPUS")
        .map_or_else(|| PathBuf::from(DEFAULT_CORPUS), PathBuf::from);
    let manifest = resolve_repo_relative(&repo_root, &manifest);
    let corpus = resolve_repo_relative(&repo_root, &corpus);
    let bytes = fs::read(&manifest).unwrap_or_else(|error| {
        panic!(
            "reading decoder benchmark golden manifest {} failed: {error}",
            manifest.display()
        )
    });
    let rows = parse_golden_manifest(&bytes).unwrap_or_else(|error| {
        panic!(
            "parsing decoder benchmark golden manifest {} failed: {error}",
            manifest.display()
        )
    });
    let explicit = env::var_os("FORNAX_BENCH_RAW")
        .map(PathBuf::from)
        .map(|path| resolve_repo_relative(&repo_root, &path));
    let inputs = select_golden_inputs(&rows, explicit.as_deref(), &corpus)
        .unwrap_or_else(|error| panic!("configuring decoder benchmark inputs failed: {error}"));
    ensure_materialized(&inputs)
        .unwrap_or_else(|error| panic!("decoder benchmark input check failed: {error}"));
    inputs
}

fn decoder_error(case: &str, decoder: &str, method: &str, error: impl std::fmt::Display) -> ! {
    panic!("decoder benchmark {decoder} failed {method} for {case}: {error}")
}

fn bench_fornax(group: &mut BenchmarkGroup<'_, WallTime>, case: &str, path: &Path) {
    group.bench_function("fornax/decode", |b| {
        b.iter(|| {
            Decoder::decode(black_box(path))
                .unwrap_or_else(|error| decoder_error(case, "fornax", "decode", error))
        });
    });

    let decoder = Decoder::decode(path)
        .unwrap_or_else(|error| decoder_error(case, "fornax", "decode", error));
    group.bench_function("fornax/raw_mosaic", |b| {
        b.iter(|| {
            decoder
                .raw_mosaic()
                .unwrap_or_else(|error| decoder_error(case, "fornax", "raw_mosaic", error))
        });
    });
    group.bench_function("fornax/cropped_mosaic", |b| {
        b.iter(|| {
            decoder
                .cropped_mosaic::<f32>()
                .unwrap_or_else(|error| decoder_error(case, "fornax", "cropped_mosaic", error))
        });
    });
}

fn bench_reference<D>(
    group: &mut BenchmarkGroup<'_, WallTime>,
    case: &str,
    path: &Path,
    decoder_name: &str,
) where
    D: IDecoder,
{
    let decoder = match preflight_reference::<D>(path) {
        Ok(ReferencePreflight::Ready(decoder)) => decoder,
        Ok(ReferencePreflight::Unsupported(detail)) => {
            eprintln!(
                "decoder benchmark warning: skipping {decoder_name} for {case}: \
                 unsupported input ({detail})"
            );
            return;
        }
        Err(error) => decoder_error(case, decoder_name, "decode/raw_mosaic preflight", error),
    };
    group.bench_function(format!("{decoder_name}/decode"), |b| {
        b.iter(|| {
            D::decode(black_box(path))
                .unwrap_or_else(|error| decoder_error(case, decoder_name, "decode", error))
        });
    });
    group.bench_function(format!("{decoder_name}/raw_mosaic"), |b| {
        b.iter(|| {
            decoder
                .raw_mosaic()
                .unwrap_or_else(|error| decoder_error(case, decoder_name, "raw_mosaic", error))
        });
    });
}

fn bench_metadata<D>(group: &mut BenchmarkGroup<'_, WallTime>, case: &str, path: &Path)
where
    D: IDecoder + IDecoderDng,
{
    let decoder =
        D::decode(path).unwrap_or_else(|error| decoder_error(case, "fornax", "decode", error));
    let metadata = decoder
        .dng_metadata()
        .unwrap_or_else(|error| decoder_error(case, "fornax", "dng_metadata", error));
    eprintln!("{case}: fornax dng_metadata tags={}", metadata.len());
    group.bench_function("fornax/dng_metadata", |b| {
        b.iter(|| {
            decoder
                .dng_metadata()
                .unwrap_or_else(|error| decoder_error(case, "fornax", "dng_metadata", error))
        });
    });
    group.bench_function("fornax/process_metadata", |b| {
        b.iter(|| {
            decoder
                .process_metadata()
                .unwrap_or_else(|error| decoder_error(case, "fornax", "process_metadata", error))
        });
    });
}

fn bench_input(c: &mut Criterion, decoders: &[DecoderKind], row: &GoldenInput, path: &Path) {
    eprintln!("decoder benchmark input: {} ({} bytes)", row.path, row.size);
    let mut group = c.benchmark_group(row.case_label());
    if decoders.contains(&DecoderKind::Fornax) {
        bench_metadata::<Decoder>(&mut group, &row.path, path);
        bench_fornax(&mut group, &row.path, path);
    }
    if decoders.contains(&DecoderKind::Rawler) {
        bench_reference::<RawlerDecoder>(&mut group, &row.path, path, "rawler");
    }
    if decoders.contains(&DecoderKind::FlashRaw) {
        bench_reference::<FlashRawDecoder>(&mut group, &row.path, path, "flashraw");
    }
    group.finish();
}

fn benches(c: &mut Criterion) {
    let decoders = selected_decoders();
    for (row, path) in benchmark_inputs() {
        bench_input(c, &decoders, &row, &path);
    }
}

criterion_group!(benches_group, benches);
criterion_main!(benches_group);
