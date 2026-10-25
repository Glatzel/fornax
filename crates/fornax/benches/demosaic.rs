use criterion::{Criterion, black_box, criterion_group, criterion_main};
use fornax_core::{BayerImage, BayerPattern, FornaxPrimitive};
use fornax_dalim::demosaic::{DemosaicLinear, IDemosaic};
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};

fn bench_methods<M, T>(c: &mut Criterion, name: &str, method: M, image: BayerImage<T>)
where
    M: IDemosaic<T>,
    T: FornaxPrimitive,
{
    c.bench_function(name, |b| b.iter(|| method.demosaic(black_box(&image))));
}
fn benches(c: &mut Criterion) {
    let mut rng = StdRng::seed_from_u64(12345);
    let buf: Vec<f32> = (0..1920 * 1080)
        .map(|_| rng.random_range(0.0..1.0))
        .collect();
    let image = BayerImage::<f32>::new(
        image::ImageBuffer::from_vec(1920, 1080, buf).unwrap(),
        BayerPattern::RGGB,
    );
    bench_methods::<_, f32>(c, "linear", DemosaicLinear, image.clone());
}
criterion_group!(benches_group, benches);
criterion_main!(benches_group);
