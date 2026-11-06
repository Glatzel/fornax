use std::io::Cursor;

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use fornax_core::{MonoImage, RawMosaic};
use fornax_devtool::{BenchParams, BenchSampleBits, synthetic_mono_image};
use fornax_dng::metadata::{DngTag, Tag, Value};
use fornax_dng::{Directory, DngCompression, DngWriter};
use fornax_tiff::constants::TiffTag;

fn metadata(width: u32, height: u32) -> Directory {
    let mut metadata = Directory::default();
    metadata.insert(
        Tag::from(DngTag::ActiveArea),
        Value::List(vec![
            Value::Unsigned(0),
            Value::Unsigned(0),
            Value::Unsigned(height),
            Value::Unsigned(width),
        ]),
    );
    metadata.insert(
        Tag::from(DngTag::DefaultCropOrigin),
        Value::List(vec![Value::Unsigned(0), Value::Unsigned(0)]),
    );
    metadata.insert(
        Tag::from(DngTag::DefaultCropSize),
        Value::List(vec![Value::Unsigned(width), Value::Unsigned(height)]),
    );
    metadata.insert(
        Tag::from(DngTag::BlackLevelRepeatDim),
        Value::List(vec![Value::Short(2), Value::Short(2)]),
    );
    metadata.insert(
        Tag::from(DngTag::BlackLevel),
        Value::List(vec![
            Value::Short(0),
            Value::Short(0),
            Value::Short(0),
            Value::Short(0),
        ]),
    );
    metadata.insert(Tag::from(DngTag::WhiteLevel), Value::Unsigned(65_535));
    metadata.insert(
        Tag::from(TiffTag::CfaRepeatPatternDim),
        Value::List(vec![Value::Short(2), Value::Short(2)]),
    );
    metadata.insert(
        Tag::from(TiffTag::CfaPattern),
        Value::List(vec![
            Value::Byte(0),
            Value::Byte(1),
            Value::Byte(1),
            Value::Byte(2),
        ]),
    );
    metadata.insert(Tag::from(DngTag::CfaLayout), Value::Short(1));
    metadata
}

fn selected_compressions() -> Vec<(String, DngCompression)> {
    let default = if cfg!(feature = "zstd") {
        "uncompressed,lossless-jpeg,deflate,zstd"
    } else {
        "uncompressed,lossless-jpeg,deflate"
    };
    let selected =
        std::env::var("FORNAX_BENCH_DNG_COMPRESSION").unwrap_or_else(|_| default.to_owned());
    selected
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| {
            let compression = match value {
                "uncompressed" => DngCompression::Uncompressed,
                "lossless-jpeg" => DngCompression::LosslessJpeg,
                "deflate" => DngCompression::Deflate,
                "zstd" if cfg!(feature = "zstd") => DngCompression::Zstd,
                _ => panic!("unknown FORNAX_BENCH_DNG_COMPRESSION value {value:?}"),
            };
            (value.to_owned(), compression)
        })
        .collect()
}

fn zstd_level() -> i32 {
    std::env::var("FORNAX_BENCH_ZSTD_LEVEL").map_or(3, |value| {
        value
            .parse()
            .expect("FORNAX_BENCH_ZSTD_LEVEL must be an integer")
    })
}

fn bench_writer(c: &mut Criterion) {
    let params = BenchParams::from_env();
    assert!(
        params.bits.iter().all(|bits| *bits == BenchSampleBits::F32),
        "DNG writer benchmarks currently accept f32 control only"
    );
    let compressions = selected_compressions();
    let level = zstd_level();
    for size in &params.sizes {
        let source = synthetic_mono_image::<f32>(size.dims, 1.0);
        let image = MonoImage::from_raw(
            size.dims.0,
            size.dims.1,
            source
                .into_raw()
                .into_iter()
                .map(|value| (value * f32::from(u16::MAX)).round() as u16)
                .collect(),
        )
        .unwrap();
        let mosaic = RawMosaic::U16(image);
        let metadata = metadata(size.dims.0, size.dims.1);
        for (name, compression) in &compressions {
            let mut output = Cursor::new(Vec::new());
            DngWriter::new(*compression)
                .with_zstd_level(level)
                .write_dng_with(&mut output, &mosaic, &metadata)
                .unwrap();
            let bytes = output.into_inner().len();
            let raw = usize::try_from(size.dims.0).unwrap()
                * usize::try_from(size.dims.1).unwrap()
                * size_of::<u16>();
            eprintln!(
                "writer/{}/{name}: {bytes} bytes, ratio {:.3}",
                size.label,
                bytes as f64 / raw as f64
            );
            c.bench_function(&format!("writer/{}/{name}", size.label), |b| {
                b.iter(|| {
                    let mut output = Cursor::new(Vec::new());
                    DngWriter::new(*compression)
                        .with_zstd_level(level)
                        .write_dng_with(
                            black_box(&mut output),
                            black_box(&mosaic),
                            black_box(&metadata),
                        )
                        .unwrap();
                    black_box(output.into_inner())
                });
            });
        }
    }
}

criterion_group!(benches, bench_writer);
criterion_main!(benches);
