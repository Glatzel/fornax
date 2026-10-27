use criterion::{Criterion, black_box, criterion_group, criterion_main};
use fornax_core::{BayerPattern, IDemosaic, MonoImage, Primitive};
use fornax_demosaic::Demosaic;
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};

fn bench_methods<M, T>(c: &mut Criterion, name: &str, method: M, image: MonoImage<T>)
where
    M: IDemosaic<T>,
    T: Primitive,
{
    c.bench_function(name, |b| {
        b.iter(|| method.demosaic(black_box(&image), black_box(BayerPattern::RGGB)))
    });
}
fn benches(c: &mut Criterion) {
    let mut rng = StdRng::seed_from_u64(12345);
    let buf: Vec<f32> = (0..1920 * 1080)
        .map(|_| rng.random_range(0.0..1.0))
        .collect();
    let image = image::ImageBuffer::from_vec(1920, 1080, buf).unwrap();
    bench_methods::<_, f32>(c, "linear", Demosaic::Linear, image.clone());
}
criterion_group!(benches_group, benches);
criterion_main!(benches_group);
