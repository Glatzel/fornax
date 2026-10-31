use bench_demosaic::{algorithm_ids, demosaic, selected_algorithm};
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use fornax_core::protocol::{IDemosaic, LazyDemosaic};
use fornax_core::{BayerPattern, MonoImage, Primitive};
use fornax_devtool::{BenchParams, BenchSampleBits, synthetic_mono_image};

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
        b.iter(|| {
            let lazy = demosaic
                .demosaic(black_box(&image), BayerPattern::RGGB)
                .unwrap();
            pool.install(|| black_box(lazy.collect()))
        });
    });
}

fn bench_sizes<T: Primitive + Send + Sync>(
    c: &mut Criterion,
    params: &BenchParams,
    bits: BenchSampleBits,
) {
    let selected = selected_algorithm();
    for (threads, pool) in params.pools() {
        for size in &params.sizes {
            let image = synthetic_mono_image::<T>(size.dims, 1.0);
            for algorithm in algorithm_ids() {
                if selected
                    .as_ref()
                    .is_some_and(|selected| selected != algorithm)
                {
                    continue;
                }
                let name = params.case_name(algorithm, size, bits, threads);
                bench_methods::<_, T>(c, &pool, &name, demosaic(algorithm), image.clone());
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
