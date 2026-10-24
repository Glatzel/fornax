use std::num::{NonZeroU32, NonZeroU64};

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use fornax::{
    ActiveArea, AuthorityClass, BayerLayout, BayerPattern, BlackLevels, CommonMetadata, Confidence,
    DecodedRaw, DecoderProvenance, DemosaicAlgorithm, DemosaicOptions, ExtensibleMetadata,
    IDemosaic, PixelSize, Provenance, ProviderId, Rational, RawImage, SampleFormat, SampleSpec,
    SourceKind, Sourced, StoredGeometry, StoredPlane, ValidatedMetadata, WhiteLevels,
    into_preprocessed,
};
use image::ImageBuffer;

fn sourced<T>(value: T, provenance: &Provenance) -> Sourced<T> {
    Sourced::new(value, provenance.clone())
}

fn image(width: u32, height: u32) -> RawImage {
    let dimensions = PixelSize::new(width, height).unwrap();
    let active_area = ActiveArea::try_new(dimensions, 0, 0, width, height).unwrap();
    let provider = ProviderId::try_new("benchmark").unwrap();
    let provenance = fornax::Provenance::new(
        provider.clone(),
        SourceKind::Derived {
            operation: "synthetic mosaic".into(),
        },
        AuthorityClass::DerivedValue,
        Confidence::Exact,
    );
    let zero = Rational::new(0, NonZeroU32::new(1).unwrap());
    let one = Rational::new(1, NonZeroU32::new(1).unwrap());
    let stride = u64::from(width) * 4;
    let metadata = ValidatedMetadata::try_new(
        CommonMetadata {
            sample_spec: Some(sourced(
                SampleSpec::try_new(32, SampleFormat::Float).unwrap(),
                &provenance,
            )),
            stored_geometry: Some(sourced(
                StoredGeometry::new(dimensions, NonZeroU64::new(stride).unwrap()),
                &provenance,
            )),
            active_area: Some(sourced(active_area, &provenance)),
            cfa: Some(sourced(
                BayerLayout::new(BayerPattern::Rggb, active_area),
                &provenance,
            )),
            black: Some(sourced(
                BlackLevels::try_new(zero, [zero; 4], None).unwrap(),
                &provenance,
            )),
            white: Some(sourced(
                WhiteLevels::try_new(vec![one], None, None).unwrap(),
                &provenance,
            )),
            ..CommonMetadata::default()
        },
        ExtensibleMetadata::default(),
        DecoderProvenance::new(provider, None),
    )
    .unwrap();
    let samples = (0..width * height)
        .map(|index| (index % 256) as f32 / 255.0)
        .collect();
    let plane = ImageBuffer::from_raw(width, height, samples).unwrap();
    let decoded = DecodedRaw::try_new(metadata, StoredPlane::F32(plane.clone())).unwrap();
    into_preprocessed(decoded, plane).unwrap()
}

fn bench_linear(c: &mut Criterion) {
    let input = image(1920, 1080);
    let algorithm = DemosaicAlgorithm::Linear;
    let options = DemosaicOptions;
    c.bench_function("linear 1920x1080 RGGB", |b| {
        b.iter(|| {
            black_box(
                algorithm
                    .demosaic(black_box(&input), black_box(&options))
                    .unwrap(),
            )
        });
    });
}

criterion_group!(benches, bench_linear);
criterion_main!(benches);
