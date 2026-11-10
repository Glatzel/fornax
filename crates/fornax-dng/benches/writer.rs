use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};
use fornax_core::{MonoImage, RawFormat, RawMosaic};
use fornax_devtool::{
    BenchParams, BenchSampleBits, DngBenchmarkInput, dng_benchmark_inputs, raw_mosaic_bytes,
    synthetic_mono_image,
};
use fornax_dng::Directory;
use fornax_dng::dng_decoder::DngImage;
use fornax_dng::metadata::{DngTag, Tag, Value};
use fornax_dng::writer::{DngCompression, DngWriter};
use fornax_tiff::constants::TiffTag;

struct BenchmarkMosaic {
    label: String,
    mosaic: RawMosaic,
}

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
    let default = "uncompressed,lossless-jpeg,deflate,zstd";
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
                "zstd" => DngCompression::Zstd,
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

fn real_mosaic(path: &Path) -> RawMosaic {
    let source = fs::read(path).unwrap_or_else(|error| {
        panic!(
            "reading DNG writer benchmark input {} failed: {error}",
            path.display()
        )
    });
    let image = DngImage::parse(&source).unwrap_or_else(|error| {
        panic!(
            "parsing DNG writer benchmark input {} failed: {error}",
            path.display()
        )
    });
   image.raw_mosaic().unwrap_or_else(|error| {
        panic!(
            "decoding DNG writer benchmark input {} failed: {error}",
            path.display()
        )
    })
}

fn benchmark_mosaics(params: &BenchParams) -> Vec<BenchmarkMosaic> {
    let explicit_real = std::env::var_os("FORNAX_BENCH_DNG_RAW")
        .filter(|path| !path.is_empty())
        .map(PathBuf::from);
    let selected = dng_benchmark_inputs(
        std::env::var_os("CI").is_some(),
        explicit_real,
        fornax_devtool::raw_file(RawFormat::Dng),
    );
    let mut mosaics = Vec::new();
    for input in selected {
        match input {
            DngBenchmarkInput::Noise => {
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
                    mosaics.push(BenchmarkMosaic {
                        label: format!("noise/{}", size.label),
                        mosaic: RawMosaic::U16(image),
                    });
                }
            }
            DngBenchmarkInput::Real(path) => {
                let label = path
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .unwrap_or("input");
                mosaics.push(BenchmarkMosaic {
                    label: format!("real/{label}"),
                    mosaic: real_mosaic(&path),
                });
            }
        }
    }
    mosaics
}

fn bench_writer(c: &mut Criterion) {
    let params = BenchParams::from_env();
    assert!(
        params.bits.iter().all(|bits| *bits == BenchSampleBits::F32),
        "DNG writer benchmarks currently accept f32 control only"
    );
    let compressions = selected_compressions();
    let level = zstd_level();
    for benchmark in benchmark_mosaics(&params) {
        let (width, height) = benchmark.mosaic.dimensions();
        let metadata = metadata(width, height);
        let raw = raw_mosaic_bytes(&benchmark.mosaic);
        let mut group = c.benchmark_group(format!("writer/{}", benchmark.label));
        for (name, compression) in &compressions {
            let mut output = Cursor::new(Vec::new());
            DngWriter::new(*compression)
                .with_zstd_level(level)
                .write_dng_with(&mut output, &benchmark.mosaic, &metadata)
                .unwrap();
            let bytes = output.into_inner().len();
            group.throughput(Throughput::Bytes(u64::try_from(bytes).unwrap()));
            eprintln!(
                "writer/{}/{name}: {bytes} bytes, ratio {:.3}",
                benchmark.label,
                bytes as f64 / raw as f64
            );
            group.bench_function(name, |b| {
                b.iter(|| {
                    let mut output = Cursor::new(Vec::new());
                    DngWriter::new(*compression)
                        .with_zstd_level(level)
                        .write_dng_with(
                            black_box(&mut output),
                            black_box(&benchmark.mosaic),
                            black_box(&metadata),
                        )
                        .unwrap();
                    black_box(output.into_inner())
                });
            });
        }
        group.finish();
    }
}

criterion_group!(benches, bench_writer);
criterion_main!(benches);
