use std::path::{Path, PathBuf};

use bench_decoder::{DecoderBenchSelection, DecoderKind};
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use fornax_core::Primitive;
use fornax_core::protocol::IDecoder;
use fornax_decoder::Decoder;
use fornax_decoder_rawler::RawlerDecoder;
use fornax_devtool::{BenchParams, BenchSampleBits};

fn raw_path() -> PathBuf {
    let path = std::env::var_os("FORNAX_BENCH_RAW").map_or_else(
        || fornax_devtool::raw_file(fornax_core::RawFormat::Dng),
        PathBuf::from,
    );
    assert!(
        path.is_file(),
        "RAW benchmark fixture does not exist: {}; set FORNAX_BENCH_RAW or run python3 scripts/setup.py",
        path.display()
    );
    path
}

fn bench_decoder<D, T>(
    c: &mut Criterion,
    decoder_name: &str,
    sample_name: &str,
    path: &Path,
) where
    D: for<'a> IDecoder<&'a Path, T>,
    T: Primitive + Send + Sync,
{
    let prefix = format!("{decoder_name}/{sample_name}");
    c.bench_function(&format!("{prefix}/decode"), |b| {
        b.iter(|| black_box(D::decode(black_box(path)).unwrap()));
    });

    let decoder = D::decode(path).unwrap();
    c.bench_function(&format!("{prefix}/raw_mosaic"), |b| {
        b.iter(|| black_box(decoder.raw_mosaic().unwrap()));
    });
    c.bench_function(&format!("{prefix}/cropped_mosaic"), |b| {
        b.iter(|| black_box(decoder.cropped_mosaic().unwrap()));
    });
    let metadata = decoder.dng_metadata().unwrap();
    eprintln!(
        "{decoder_name}/{sample_name}: dng_metadata tags={}",
        metadata.len()
    );
    c.bench_function(&format!("{prefix}/dng_metadata"), |b| {
        b.iter(|| black_box(decoder.dng_metadata().unwrap()));
    });
    c.bench_function(&format!("{prefix}/process_metadata"), |b| {
        b.iter(|| black_box(decoder.process_metadata().unwrap()));
    });
    c.bench_function(&format!("{prefix}/decode+raw_mosaic"), |b| {
        b.iter(|| {
            let decoder = D::decode(black_box(path)).unwrap();
            black_box(decoder.raw_mosaic().unwrap())
        });
    });
    c.bench_function(&format!("{prefix}/decode+cropped_mosaic"), |b| {
        b.iter(|| {
            let decoder = D::decode(black_box(path)).unwrap();
            black_box(decoder.cropped_mosaic().unwrap())
        });
    });
}

fn bench_bit<D, T>(
    c: &mut Criterion,
    decoder_name: &str,
    bits: BenchSampleBits,
    path: &Path,
) where
    D: for<'a> IDecoder<&'a Path, T>,
    T: Primitive + Send + Sync,
{
    bench_decoder::<D, T>(c, decoder_name, bits.as_ref(), path);
}

fn benches(c: &mut Criterion) {
    let params = BenchParams::from_env();
    let decoders = DecoderBenchSelection::from_env().unwrap();
    let path = raw_path();

    for bits in params.bits {
        for decoder in &decoders {
            match (decoder, bits) {
                (DecoderKind::Fornax, BenchSampleBits::F32) => {
                    bench_bit::<Decoder<f32>, f32>(c, "fornax", bits, &path);
                }
                (DecoderKind::Fornax, BenchSampleBits::F64) => {
                    bench_bit::<Decoder<f64>, f64>(c, "fornax", bits, &path);
                }
                (DecoderKind::Rawler, BenchSampleBits::F32) => {
                    bench_bit::<RawlerDecoder<f32>, f32>(c, "rawler", bits, &path);
                }
                (DecoderKind::Rawler, BenchSampleBits::F64) => {
                    bench_bit::<RawlerDecoder<f64>, f64>(c, "rawler", bits, &path);
                }
            }
        }
    }
}

criterion_group!(benches_group, benches);
criterion_main!(benches_group);
