use criterion::{BatchSize, Criterion, black_box, criterion_group, criterion_main};
use fornax_core::error::PreprocessError;
use fornax_core::metadata::Levels;
use fornax_core::{DecodedRawActive, IPreprocessor, MonoImage, Primitive};
use fornax_devtool::{BenchSampleBits, bench_mono_image, bench_sample_bits};
use fornax_preprocessor::Preprocessor;

fn bench_inputs<T: Primitive>(c: &mut Criterion, sample_bits: BenchSampleBits) {
    let levels = Levels {
        black: MonoImage::from_raw(2, 2, vec![T::zero(); 4]).expect("2x2 black-level pattern"),
        black_delta_h: Vec::new(),
        black_delta_v: Vec::new(),
        white: T::one(),
    };

    for input in fornax_devtool::bench_inputs() {
        let source = bench_mono_image::<T>(input, 1.0);
        let name = format!("preprocess/{}/{}", input.name, sample_bits.name());
        c.bench_function(&name, |b| {
            b.iter_batched(
                || DecodedRawActive::new(black_box(source.clone())),
                |decoded| -> Result<_, PreprocessError> {
                    Preprocessor.preprocess(decoded, black_box(&levels))
                },
                BatchSize::LargeInput,
            )
        });
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
