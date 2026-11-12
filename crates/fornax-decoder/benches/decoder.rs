mod decoder_inputs;
mod flashraw_wrapper;
mod rawler_wrapper;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use criterion::measurement::WallTime;
use criterion::{BenchmarkGroup, Criterion, black_box, criterion_group, criterion_main};
use decoder_inputs::{GoldenInput, ensure_materialized, parse_golden_manifest, select_golden_inputs};
use flashraw_wrapper::FlashRawDecoder;
use fornax_decoder::Decoder;
use fornax_devtool::{BenchParams, BenchSampleBits};
use fornax_dng::metadata::IDecoderDng;
use fornax_raw_core::Primitive;
use fornax_raw_core::protocol::IDecoder;
use rawler_wrapper::RawlerDecoder;

const DEFAULT_MANIFEST: &str = "crates/decoder-tracker/golden/selected-manifest.csv";
const DEFAULT_CORPUS: &str = "external/raw.pixls.us";

#[derive(Clone, Copy, Eq, PartialEq)]
enum DecoderKind {
    Fornax,
    Rawler,
    FlashRaw,
}

fn selected_decoders() -> Vec<DecoderKind> {
    let value =
        env::var("FORNAX_BENCH_DECODER").unwrap_or_else(|_| "fornax,rawler,flashraw".into());
    value
        .split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(|name| match name {
            "fornax" => DecoderKind::Fornax,
            "rawler" => DecoderKind::Rawler,
            "flashraw" => DecoderKind::FlashRaw,
            _ => panic!(
                "unknown FORNAX_BENCH_DECODER value {name:?}; expected fornax, rawler, or flashraw"
            ),
        })
        .collect()
}

fn benchmark_inputs() -> Vec<(GoldenInput, PathBuf)> {
    let manifest = env::var_os("FORNAX_BENCH_MANIFEST")
        .map_or_else(|| PathBuf::from(DEFAULT_MANIFEST), PathBuf::from);
    let corpus = env::var_os("FORNAX_BENCH_CORPUS")
        .map_or_else(|| PathBuf::from(DEFAULT_CORPUS), PathBuf::from);
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
    let explicit = env::var_os("FORNAX_BENCH_RAW").map(PathBuf::from);
    let inputs = select_golden_inputs(&rows, explicit.as_deref(), &corpus)
        .unwrap_or_else(|error| panic!("configuring decoder benchmark inputs failed: {error}"));
    ensure_materialized(&inputs)
        .unwrap_or_else(|error| panic!("decoder benchmark input check failed: {error}"));
    inputs
}

fn decoder_error(case: &str, decoder: &str, method: &str, error: impl std::fmt::Display) -> ! {
    panic!("decoder benchmark {decoder} failed {method} for {case}: {error}")
}

fn bench_decoder<D, T>(
    group: &mut BenchmarkGroup<'_, WallTime>,
    case: &str,
    path: &Path,
    decoder_name: &str,
    sample_name: &str,
) where
    D: IDecoder,
    T: Primitive + Send + Sync,
{
    let prefix = format!("{decoder_name}/{sample_name}");
    group.bench_function(format!("{prefix}/decode"), |b| {
        b.iter(|| {
            D::decode(black_box(path)).unwrap_or_else(|error| {
                decoder_error(case, decoder_name, "decode", error)
            })
        });
    });

    let decoder = D::decode(path)
        .unwrap_or_else(|error| decoder_error(case, decoder_name, "decode", error));
    group.bench_function(format!("{prefix}/raw_mosaic"), |b| {
        b.iter(|| {
            decoder.raw_mosaic().unwrap_or_else(|error| {
                decoder_error(case, decoder_name, "raw_mosaic", error)
            })
        });
    });
    group.bench_function(format!("{prefix}/cropped_mosaic"), |b| {
        b.iter(|| {
            decoder.cropped_mosaic::<T>().unwrap_or_else(|error| {
                decoder_error(case, decoder_name, "cropped_mosaic", error)
            })
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
            decoder.process_metadata().unwrap_or_else(|error| {
                decoder_error(case, "fornax", "process_metadata", error)
            })
        });
    });
}

fn bench_input(
    c: &mut Criterion,
    params: &BenchParams,
    decoders: &[DecoderKind],
    row: &GoldenInput,
    path: &Path,
) {
    eprintln!(
        "decoder benchmark input: {} ({} bytes)",
        row.path, row.size
    );
    let mut group = c.benchmark_group(row.case_label());
    if decoders.contains(&DecoderKind::Fornax) {
        bench_metadata::<Decoder>(&mut group, &row.path, path);
    }

    for bits in &params.bits {
        for decoder in decoders {
            match (decoder, bits) {
                (DecoderKind::Fornax, BenchSampleBits::F32) => {
                    bench_decoder::<Decoder, f32>(
                        &mut group,
                        &row.path,
                        path,
                        "fornax",
                        bits.as_ref(),
                    );
                }
                (DecoderKind::Fornax, BenchSampleBits::F64) => {
                    bench_decoder::<Decoder, f64>(
                        &mut group,
                        &row.path,
                        path,
                        "fornax",
                        bits.as_ref(),
                    );
                }
                (DecoderKind::Rawler, BenchSampleBits::F32) => {
                    bench_decoder::<RawlerDecoder, f32>(
                        &mut group,
                        &row.path,
                        path,
                        "rawler",
                        bits.as_ref(),
                    );
                }
                (DecoderKind::Rawler, BenchSampleBits::F64) => {
                    bench_decoder::<RawlerDecoder, f64>(
                        &mut group,
                        &row.path,
                        path,
                        "rawler",
                        bits.as_ref(),
                    );
                }
                (DecoderKind::FlashRaw, BenchSampleBits::F32) => {
                    bench_decoder::<FlashRawDecoder, f32>(
                        &mut group,
                        &row.path,
                        path,
                        "flashraw",
                        bits.as_ref(),
                    );
                }
                (DecoderKind::FlashRaw, BenchSampleBits::F64) => {
                    bench_decoder::<FlashRawDecoder, f64>(
                        &mut group,
                        &row.path,
                        path,
                        "flashraw",
                        bits.as_ref(),
                    );
                }
            }
        }
    }
    group.finish();
}

fn benches(c: &mut Criterion) {
    let params = BenchParams::from_env();
    let decoders = selected_decoders();
    for (row, path) in benchmark_inputs() {
        bench_input(c, &params, &decoders, &row, &path);
    }
}

criterion_group!(benches_group, benches);
criterion_main!(benches_group);
