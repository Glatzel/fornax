use criterion::{BatchSize, Criterion, black_box, criterion_group, criterion_main};
use fornax_core::error::PreprocessError;
use fornax_core::metadata::Levels;
use fornax_core::protocol::IPreprocessor;
use fornax_core::{DecodedRawActive, MonoImage, Primitive};
use fornax_devtool::{BenchParams, BenchSampleBits, synthetic_mono_image};
use fornax_preprocessor::Preprocessor;

fn bench_sizes<T: Primitive>(c: &mut Criterion, params: &BenchParams, bits: BenchSampleBits) {
    let levels = Levels {
        black: MonoImage::from_raw(2, 2, vec![T::zero(); 4]).expect("2x2 black-level pattern"),
        black_delta_h: Vec::new(),
        black_delta_v: Vec::new(),
        white: T::one(),
    };

    for size in &params.sizes {
        let source = synthetic_mono_image::<T>(size.dims, 1.0);
        let name = format!("preprocess/{}/{}", size.label, bits.as_ref());
        c.bench_function(&name, |b| {
            b.iter_batched(
                || DecodedRawActive::new(black_box(source.clone())),
                |decoded| -> Result<_, PreprocessError> {
                    Preprocessor.process(decoded, black_box(&levels))
                },
                BatchSize::LargeInput,
            )
        });
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
