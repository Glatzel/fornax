use criterion::{Criterion, black_box, criterion_group, criterion_main};
use fornax::{BayerPattern, demosaic};
use image::ImageBuffer;

fn benchmark_image(width: u32, height: u32) -> ImageBuffer<image::Luma<f32>, Vec<f32>> {
    let samples = (0..width * height)
        .map(|index| (index % 256) as f32 / 255.0)
        .collect();
    ImageBuffer::from_raw(width, height, samples).unwrap()
}

fn bench_linear(criterion: &mut Criterion) {
    let input = benchmark_image(1920, 1080);
    criterion.bench_function("linear 1920x1080 RGGB", |bencher| {
        bencher.iter(|| black_box(demosaic(black_box(&input), BayerPattern::Rggb)));
    });
}

criterion_group!(benches, bench_linear);
criterion_main!(benches);
