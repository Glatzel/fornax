use criterion::{Criterion, black_box, criterion_group, criterion_main};
use fornax_core::protocol::{IDemosaic, LazyDemosaic};
use fornax_core::{BayerPattern, MonoImage, Primitive};
use fornax_devtool::{BenchSampleBits, bench_bits, bench_size, synthetic_mono_image};
use fornax_postprocessor::Demosaic;

fn bench_methods<D, T>(
    c: &mut Criterion,
    pool: &rayon::ThreadPool,
    name: &str,
    demosaic: D,
    image: MonoImage<T>,
) where
    D: IDemosaic<T>,
    T: Primitive + Send + Sync,
{
    c.bench_function(name, |b| {
        let lazy = demosaic.demosaic(&image, BayerPattern::RGGB).unwrap();
        b.iter(|| pool.install(|| black_box(lazy.collect())));
    });
}
fn bench_sizes<T: Primitive + Send + Sync>(c: &mut Criterion, sample_bits: BenchSampleBits) {
    for threads in fornax_devtool::thread_counts() {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .unwrap();

        for (label, size) in bench_size() {
            let image = synthetic_mono_image::<T>(size, 1.0);
            let name = format!("linear/{label}/{}/{threads}T", sample_bits.name());
            bench_methods::<_, T>(c, &pool, &name, Demosaic::Linear, image);
        }
    }
}
fn benches(c: &mut Criterion) {
    for sample_bits in bench_bits() {
        match sample_bits {
            BenchSampleBits::B32 => bench_sizes::<f32>(c, sample_bits),
            BenchSampleBits::B64 => bench_sizes::<f64>(c, sample_bits),
        }
    }
}

criterion_group!(benches_group, benches);
criterion_main!(benches_group);
