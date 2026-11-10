mod rawler_wrapper;
use std::path::{Path, PathBuf};

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use fornax_raw_core::Primitive;
use fornax_raw_core::protocol::IDecoder;
use fornax_decoder::Decoder;
use fornax_devtool::{BenchParams, BenchSampleBits};
use fornax_dng::metadata::IDecoderDng;
use rawler_wrapper::RawlerDecoder;

#[derive(Clone, Copy, Eq, PartialEq)]
enum DecoderKind {
    Fornax,
    Rawler,
}

fn selected_decoders() -> Vec<DecoderKind> {
    let value = std::env::var("FORNAX_BENCH_DECODER").unwrap_or_else(|_| "fornax,rawler".into());
    value
        .split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(|name| match name {
            "fornax" => DecoderKind::Fornax,
            "rawler" => DecoderKind::Rawler,
            _ => panic!("unknown FORNAX_BENCH_DECODER value {name:?}; expected fornax or rawler"),
        })
        .collect()
}

fn raw_path() -> PathBuf {
    let path = std::env::var_os("FORNAX_BENCH_RAW").map_or_else(
        || fornax_devtool::raw_file(fornax_raw_core::RawFormat::Dng),
        PathBuf::from,
    );
    assert!(
        path.is_file(),
        "RAW benchmark fixture does not exist: {}; set FORNAX_BENCH_RAW or run python3 scripts/setup.py",
        path.display()
    );
    path
}

fn bench_decoder<D, T>(c: &mut Criterion, decoder_name: &str, sample_name: &str, path: &Path)
where
    D: IDecoder,
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
        b.iter(|| black_box(decoder.cropped_mosaic::<T>().unwrap()));
    });
}
fn bench_metadata<D>(c: &mut Criterion, decoder_name: &str, path: &Path)
where
    D: IDecoder + IDecoderDng,
{
    let decoder = D::decode(path).unwrap();
    let metadata = decoder.dng_metadata().unwrap();
    eprintln!("{decoder_name}: dng_metadata tags={}", metadata.len());
    c.bench_function(&format!("{decoder_name}/dng_metadata"), |b| {
        b.iter(|| black_box(decoder.dng_metadata().unwrap()));
    });
    c.bench_function(&format!("{decoder_name}/process_metadata"), |b| {
        b.iter(|| black_box(decoder.process_metadata().unwrap()));
    });
}
fn bench_bit_decoder<D, T>(
    c: &mut Criterion,
    decoder_name: &str,
    bits: BenchSampleBits,
    path: &Path,
) where
    D: IDecoder,
    T: Primitive + Send + Sync,
{
    bench_decoder::<D, T>(c, decoder_name, bits.as_ref(), path);
}

fn benches(c: &mut Criterion) {
    let params = BenchParams::from_env();
    let decoders = selected_decoders();
    let path = raw_path();

    if decoders.contains(&DecoderKind::Fornax) {
        bench_metadata::<Decoder>(c, "fornax", &path);
    }

    for bits in params.bits {
        for decoder in &decoders {
            match (decoder, bits) {
                (DecoderKind::Fornax, BenchSampleBits::F32) => {
                    bench_bit_decoder::<Decoder, f32>(c, "fornax", bits, &path);
                }
                (DecoderKind::Fornax, BenchSampleBits::F64) => {
                    bench_bit_decoder::<Decoder, f64>(c, "fornax", bits, &path);
                }
                (DecoderKind::Rawler, BenchSampleBits::F32) => {
                    bench_bit_decoder::<RawlerDecoder, f32>(c, "rawler", bits, &path);
                }
                (DecoderKind::Rawler, BenchSampleBits::F64) => {
                    bench_bit_decoder::<RawlerDecoder, f64>(c, "rawler", bits, &path);
                }
            }
        }
    }
}

criterion_group!(benches_group, benches);
criterion_main!(benches_group);
