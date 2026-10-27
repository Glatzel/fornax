use criterion::{Criterion, black_box, criterion_group, criterion_main};
use fornax_core::{BayerPattern, IDemosaic, MonoImage, Primitive};
use fornax_demosaic::Demosaic;
use fornax_devtool::{BenchSampleBits, bench_mono_image, bench_sample_bits};

fn bench_methods<M, T>(c: &mut Criterion, name: &str, method: M, image: MonoImage<T>)
where
    M: IDemosaic<T>,
    T: Primitive,
{
    c.bench_function(name, |b| {
        b.iter(|| method.demosaic(black_box(&image), black_box(BayerPattern::RGGB)))
    });
}

fn bench_inputs<T: Primitive>(c: &mut Criterion, sample_bits: BenchSampleBits) {
    for input in fornax_devtool::bench_inputs() {
        let image = bench_mono_image::<T>(input, 1.0);
        let name = format!("linear/{}/{}", input.name, sample_bits.name());
        bench_methods::<_, T>(c, &name, Demosaic::Linear, image);
    }
}

fn benches(c: &mut Criterion) {
    for sample_bits in bench_sample_bits() {
        match sample_bits {
            BenchSampleBits::B32 => bench_inputs::<f32>(c, sample_bits),
            BenchSampleBits::B64 => bench_inputs::<f64>(c, sample_bits),
        }
    }
}

criterion_group!(benches_group, benches);
criterion_main!(benches_group);
