use criterion::{Criterion, black_box, criterion_group, criterion_main};
use fornax_core::protocol::IDemosaic;
use fornax_core::{BayerPattern, MonoImage, Primitive};
use fornax_devtool::{BenchSampleBits, bench_bits, bench_size, synthetic_mono_image};
use fornax_postprocessor::Demosaic;

fn bench_methods<M, T>(c: &mut Criterion, name: &str, method: M, image: MonoImage<T>)
where
    M: IDemosaic<T>,
    T: Primitive,
{
    c.bench_function(name, |b| {
        b.iter(|| method.demosaic(black_box(&image), black_box(BayerPattern::RGGB)))
    });
}

fn bench_sizes<T: Primitive>(c: &mut Criterion, sample_bits: BenchSampleBits) {
    for (label, size) in bench_size() {
        let image = synthetic_mono_image::<T>(size, 1.0);
        let name = format!("linear/{label}/{}", sample_bits.name());
        bench_methods::<_, T>(c, &name, Demosaic::Linear, image);
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
