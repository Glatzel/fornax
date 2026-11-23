use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

mod support;

use criterion::{Criterion, Throughput, black_box, criterion_group, criterion_main};
use fornax_devtool::{BenchParams, BenchSampleBits};
use fornax_raw::dng::dng_decoder::DngImage;
use fornax_raw::dng::metadata::{DngTag, Tag, Value};
use fornax_raw::dng::writer::{DngWriter, DngWriterCompression};
use fornax_raw::dng::{Directory};
use fornax_raw::{MonoImage, RawFormat, RawMosaic};
use fornax_tiff::constants::TiffTag;
use support::{
    DngBenchmarkInput, dng_benchmark_inputs, raw_file, raw_mosaic_bytes, synthetic_mono_image,
};

#[derive(Debug, Clone)]
struct BenchmarkMosaic {
    label: String,
    mosaic: RawMosaic,
}
impl BenchmarkMosaic {
    pub fn mosaic(&self) -> RawMosaic { self.mosaic.clone() }
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

fn selected_compressions(_zstd_level: i32) -> Vec<(String, DngWriterCompression)> {
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
                "uncompressed" => DngWriterCompression::Uncompressed,
                "lossless-jpeg" => DngWriterCompression::Lossless,
                "deflate" => DngWriterCompression::Deflate,
                #[cfg(feature = "zstd")]
                "zstd" => DngWriterCompression::Zstd { level: _zstd_level },
                #[cfg(not(feature = "zstd"))]
                "zstd" => panic!("zstd support requires the fornax-raw zstd feature"),
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
        raw_file(RawFormat::Dng),
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
    let level = zstd_level();
    let compressions = selected_compressions(level);
    for benchmark in benchmark_mosaics(&params) {
        let (width, height) = benchmark.mosaic.dimensions();
        let metadata = metadata(width, height);
        let raw = raw_mosaic_bytes(&benchmark.mosaic);
        let mut group = c.benchmark_group(format!("writer/{}", benchmark.label));
        for (name, compression) in &compressions {
            let mut output = Cursor::new(Vec::new());
            DngWriter::new(*compression)
                .write_dng_with(&mut output, benchmark.clone().mosaic, &metadata)
                .unwrap();
            let bytes = output.into_inner().len();
            group.throughput(Throughput::Bytes(u64::try_from(bytes).unwrap()));
            eprintln!(
                "writer/{}/{name}: {bytes} bytes, ratio {:.3}",
                benchmark.label,
                bytes as f64 / raw as f64
            );

            group.bench_function(name, |b| {
                let mosaic = benchmark.mosaic();

                b.iter_batched(
                    || mosaic.clone(),
                    |mosaic| {
                        let mut output = Cursor::new(Vec::new());

                        DngWriter::new(*compression)
                            .write_dng_with(
                                black_box(&mut output),
                                black_box(mosaic),
                                black_box(&metadata),
                            )
                            .unwrap();

                        black_box(output.into_inner())
                    },
                    criterion::BatchSize::LargeInput,
                );
            });
        }
        group.finish();
    }
}

criterion_group!(benches, bench_writer);
criterion_main!(benches);
