mod decoder_inputs;
mod decoder_selection;
mod flashraw_wrapper;
#[path = "../tests/support/mosaic_compare.rs"]
mod mosaic_compare;
mod rawler_wrapper;
mod reference_preflight;

use std::env;
use std::path::Path;

use criterion::measurement::WallTime;
use criterion::{BenchmarkGroup, Criterion, black_box, criterion_group, criterion_main};
use decoder_inputs::{GoldenInput, benchmark_inputs};
use decoder_selection::{DecoderKind, select_decoders};
use flashraw_wrapper::FlashRawDecoder;
use fornax_dng::metadata::IDecoderDng;
use fornax_raw_core::protocol::IDecoder;
use fornax_raw_decoder::Decoder;
use mosaic_compare::{assert_mosaic_eq, canonical_mosaic};
use rawler_wrapper::RawlerDecoder;
use reference_preflight::{ReferencePreflight, preflight_reference};

fn selected_decoders() -> Vec<DecoderKind> {
    let explicit = env::var("FORNAX_BENCH_DECODER").ok();
    select_decoders(explicit.as_deref(), env::var_os("CI").is_some())
        .unwrap_or_else(|error| panic!("parsing FORNAX_BENCH_DECODER failed: {error}"))
}

fn decoder_error(case: &str, decoder: &str, method: &str, error: impl std::fmt::Display) -> ! {
    panic!("decoder benchmark {decoder} failed {method} for {case}: {error}")
}

fn bench_fornax(group: &mut BenchmarkGroup<'_, WallTime>, case: &str, path: &Path) {
    group.bench_function("fornax/end_to_end", |b| {
        b.iter(|| {
            let decoder = Decoder::decode(black_box(path))
                .unwrap_or_else(|error| decoder_error(case, "fornax", "decode", error));
            decoder
                .raw_mosaic()
                .unwrap_or_else(|error| decoder_error(case, "fornax", "raw_mosaic", error))
        });
    });

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

fn verify_three_way_equality(case: &str, path: &Path) {
    let fornax = Decoder::decode(path)
        .and_then(|decoder| decoder.raw_mosaic())
        .unwrap_or_else(|error| decoder_error(case, "fornax", "raw_mosaic", error));
    let rawler = RawlerDecoder::decode(path)
        .and_then(|decoder| decoder.raw_mosaic())
        .unwrap_or_else(|error| decoder_error(case, "rawler", "raw_mosaic", error));
    let flashraw = FlashRawDecoder::decode(path)
        .and_then(|decoder| decoder.raw_mosaic())
        .unwrap_or_else(|error| decoder_error(case, "flashraw", "raw_mosaic", error));

    let fornax = canonical_mosaic(&fornax);
    assert_mosaic_eq(case, "rawler", &canonical_mosaic(&rawler), &fornax);
    assert_mosaic_eq(case, "flashraw", &canonical_mosaic(&flashraw), &fornax);
}

fn bench_reference<D>(
    group: &mut BenchmarkGroup<'_, WallTime>,
    case: &str,
    path: &Path,
    decoder_name: &str,
) where
    D: IDecoder,
{
    match preflight_reference::<D>(path) {
        Ok(ReferencePreflight::Ready(_)) => {}
        Ok(ReferencePreflight::Unsupported(detail)) => {
            eprintln!(
                "decoder benchmark warning: skipping {decoder_name} for {case}: \
                 unsupported input ({detail})"
            );
            return;
        }
        Err(error) => decoder_error(case, decoder_name, "decode/raw_mosaic preflight", error),
    }
    group.bench_function(format!("{decoder_name}/decode"), |b| {
        b.iter(|| {
            D::decode(black_box(path))
                .unwrap_or_else(|error| decoder_error(case, decoder_name, "decode", error))
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
    if row.format() == "DNG"
        && decoders.len() == 3
        && decoders.contains(&DecoderKind::Fornax)
        && decoders.contains(&DecoderKind::Rawler)
        && decoders.contains(&DecoderKind::FlashRaw)
    {
        verify_three_way_equality(&row.path, path);
    }
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
