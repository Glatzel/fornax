use std::path::{Path, PathBuf};

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use fornax_core::Primitive;
use fornax_core::protocol::IDecoder;
use fornax_decoder::Decoder;
use fornax_decoder_rawler::RawlerDecoder;

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

fn bench_decoder<'a, D, T>(c: &mut Criterion, name: &str, path: &'a Path)
where
    D: IDecoder<&'a Path, T>,
    T: Primitive + Send + Sync,
{
    c.bench_function(&format!("{name}/decode"), |b| {
        b.iter(|| black_box(D::decode(black_box(path)).unwrap()));
    });

    let decoder = D::decode(path).unwrap();
    c.bench_function(&format!("{name}/raw_mosaic"), |b| {
        b.iter(|| black_box(decoder.raw_mosaic().unwrap()));
    });
    c.bench_function(&format!("{name}/cropped_mosaic"), |b| {
        b.iter(|| black_box(decoder.cropped_mosaic().unwrap()));
    });
    c.bench_function(&format!("{name}/dng_metadata"), |b| {
        b.iter(|| black_box(decoder.dng_metadata().unwrap()));
    });
    c.bench_function(&format!("{name}/process_metadata"), |b| {
        b.iter(|| black_box(decoder.process_metadata().unwrap()));
    });
}

fn benches(c: &mut Criterion) {
    let path = raw_path();
    bench_decoder::<Decoder<f32>, f32>(c, "fornax/f32", &path);
    bench_decoder::<Decoder<f64>, f64>(c, "fornax/f64", &path);
    bench_decoder::<RawlerDecoder<f32>, f32>(c, "rawler/f32", &path);
    bench_decoder::<RawlerDecoder<f64>, f64>(c, "rawler/f64", &path);
}

criterion_group!(benches_group, benches);
criterion_main!(benches_group);
