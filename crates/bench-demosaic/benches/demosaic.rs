use criterion::{Criterion, black_box, criterion_group, criterion_main};
use fornax_core::protocol::{IDemosaic, LazyDemosaic};
use fornax_core::{BayerPattern, MonoImage, Primitive};
use fornax_devtool::{BenchParams, BenchSampleBits, synthetic_mono_image};
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

fn bench_sizes<T: Primitive + Send + Sync>(
    c: &mut Criterion,
    params: &BenchParams,
    bits: BenchSampleBits,
) {
    for (threads, pool) in params.pools() {
        for size in &params.sizes {
            let image = synthetic_mono_image::<T>(size.dims, 1.0);
            for (prefix, demosaic) in [("linear", Demosaic::Linear), ("ppg", Demosaic::Ppg)] {
                let name = params.case_name(prefix, size, bits, threads);
                bench_methods::<_, T>(c, &pool, &name, demosaic, image.clone());
            }
        }
    }
}

fn benches(c: &mut Criterion) {
    let params = BenchParams::from_env();
    for &bits in &params.bits {
        match bits {
            BenchSampleBits::F32 => bench_sizes::<f32>(c, &params, bits),
            BenchSampleBits::F64 => bench_sizes::<f64>(c, &params, bits),
        }
    }
}

criterion_group!(benches_group, benches);
criterion_main!(benches_group);
