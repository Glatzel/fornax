use criterion::{Criterion, black_box, criterion_group, criterion_main};
use fornax_raw_core::{BayerPattern, MonoImage, Primitive};
use fornax_devtool::{BenchParams, BenchSampleBits, synthetic_mono_image};
use fornax_raw_pipeline::{Demosaic, IDemosaic, LazyDemosaic};
fn selected_algorithm() -> Vec<(String, Demosaic)> {
    const ALL: &[&str] = &[
        "aahd", "ahd", "amaze", "dcb", "dht", "linear", "lmmse", "ppg", "rcd", "vng", "vng4",
    ];

    let algorithms = std::env::var("FORNAX_BENCH_ALGORITHM").unwrap_or_else(|_| ALL.join(","));

    algorithms
        .split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(|name| {
            let algorithm = match name {
                "aahd" => Demosaic::Aahd,
                "ahd" => Demosaic::Ahd,
                "amaze" => Demosaic::Amaze,
                "dcb" => Demosaic::dcb(),
                "dht" => Demosaic::Dht,
                "linear" => Demosaic::Linear,
                "lmmse" => Demosaic::lmmse(),
                "ppg" => Demosaic::Ppg,
                "rcd" => Demosaic::Rcd,
                "vng" => Demosaic::Vng,
                "vng4" => Demosaic::Vng4,
                _ => panic!("Unknown demosaic algorithm: {name}"),
            };

            (name.to_owned(), algorithm)
        })
        .collect()
}
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

fn bench_sizes<T>(c: &mut Criterion, params: &BenchParams, bits: BenchSampleBits)
where
    T: Primitive + Send + Sync,
{
    for (threads, pool) in params.pools() {
        for size in &params.sizes {
            let image = synthetic_mono_image::<T>(size.dims, 1.0);
            for (algorithm_name, algorithm) in selected_algorithm() {
                let name = params.case_name(&algorithm_name, size, bits, threads);
                bench_methods::<_, T>(c, &pool, &name, algorithm, image.clone());
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
