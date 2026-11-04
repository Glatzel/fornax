use std::path::{Path, PathBuf};

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use fornax_core::protocol::IDecoder;
use fornax_core::{Decoder, Primitive};

fn raw_path() -> PathBuf {
    let path = std::env::var_os("FORNAX_BENCH_RAW")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            fornax_devtool::root_dir().join("external/raw-images/images/colorchart-5D2-6000K.dng")
        });
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
    bench_decoder::<_, f32>(c, "full/f32", Decoder, &path);
    bench_decoder::<_, f64>(c, "full/f64", Decoder, &path);
}

criterion_group!(benches_group, benches);
criterion_main!(benches_group);
