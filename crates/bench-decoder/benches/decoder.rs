use std::path::{Path, PathBuf};

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use fornax_core::Primitive;
use fornax_core::protocol::IDecoder;
use fornax_decoder_rawler::RawlerDecoder;
use libraw::LibrawDecoder;

fn raw_path() -> PathBuf {
    let path = std::env::var_os("FORNAX_BENCH_RAW")
        .map(PathBuf::from)
        .unwrap_or_else(fornax_devtool::raw_file);
    assert!(
        path.is_file(),
        "RAW benchmark fixture does not exist: {}; set FORNAX_BENCH_RAW or run python3 scripts/setup.py",
        path.display()
    );
    path
}

fn bench_decoder<D, T>(c: &mut Criterion, name: &str, decoder: D, path: &Path)
where
    D: IDecoder<Path, T>,
    T: Primitive + Send + Sync,
{
    c.bench_function(name, |b| {
        b.iter(|| black_box(decoder.decode_full(black_box(path)).unwrap()));
    });
}

fn benches(c: &mut Criterion) {
    let path = raw_path();
    bench_decoder::<_, f32>(c, "rawler/full/f32", RawlerDecoder, &path);
    bench_decoder::<_, f64>(c, "rawler/full/f64", RawlerDecoder, &path);
    bench_decoder::<_, f32>(c, "libraw/full/f32", LibrawDecoder, &path);
    bench_decoder::<_, f64>(c, "libraw/full/f64", LibrawDecoder, &path);
}

criterion_group!(benches_group, benches);
criterion_main!(benches_group);
