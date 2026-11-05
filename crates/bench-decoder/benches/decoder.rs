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

fn bench_decoder<'a, D, T>(c: &mut Criterion, name: &str, decoder: D, path: &'a Path)
where
    D: IDecoder<&'a Path, T>,
    T: Primitive + Send + Sync,
{
    c.bench_function(name, |b| {
        b.iter(|| black_box(decoder.decode(black_box(path)).unwrap()));
    });
}

fn benches(c: &mut Criterion) {
    let path = raw_path();
    bench_decoder::<_, f32>(c, "fornax/decode/f32", Decoder, &path);
    bench_decoder::<_, f64>(c, "fornax/decode/f64", Decoder, &path);
    bench_decoder::<_, f32>(c, "rawler/decode/f32", RawlerDecoder, &path);
    bench_decoder::<_, f64>(c, "rawler/decode/f64", RawlerDecoder, &path);
}

criterion_group!(benches_group, benches);
criterion_main!(benches_group);
