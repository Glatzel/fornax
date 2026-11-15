#[path = "../flashraw_wrapper.rs"]
mod flashraw_wrapper;
#[path = "../rawler_wrapper.rs"]
mod rawler_wrapper;

use std::env;
use std::fs::{self, File};
use std::hint::black_box;
use std::mem::size_of;
use std::path::{Path, PathBuf};
use std::ptr::NonNull;
use std::time::{Duration, Instant};

use flashraw_wrapper::FlashRawDecoder;
use fornax_codec::{
    ByteOrder, Chunk, DecodeCompression, DecodeRequest, ICodecDecode, Predictor, SampleFormat,
};
use fornax_dng::dng_decoder::DngImage;
use fornax_raw_core::RawMosaic;
use fornax_raw_core::protocol::IDecoder;
use fornax_raw_decoder::Decoder;
use fornax_tiff::constants::TiffTag;
use fornax_tiff::{TiffSource, unsigned_scalar, unsigned_vec};
use memmap2::Mmap;
use rawler_wrapper::RawlerDecoder;

const USAGE: &str = "\
usage:
  dng_gap_probe equality PATH
  dng_gap_probe decode --decoder fornax|flashraw --warmups N --rounds N \
    [--threads 1|all] [--smaps] PATH
  dng_gap_probe uncompressed \
    --variant fresh|reused|mmap-hugepage|alloc-zeroed-hugepage|vec-populate|vec-hugepage|\
vec-hugepage-populate \
    [--counter-only] --rounds N PATH
";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ProbeDecoder {
    Fornax,
    FlashRaw,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ProbeMode {
    Decode,
    Equality,
    Uncompressed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum UncompressedVariant {
    Fresh,
    Reused,
    MmapHugePage,
    AllocZeroedHugePage,
    VecPopulate,
    VecHugePage,
    VecHugePagePopulate,
}

#[derive(Debug, Eq, PartialEq)]
struct ProbeArgs {
    mode: ProbeMode,
    decoder: ProbeDecoder,
    rounds: usize,
    warmups: usize,
    variant: UncompressedVariant,
    counter_only: bool,
    all_threads: bool,
    inspect_huge_pages: bool,
    input: PathBuf,
}

pub(super) fn main() {
    if let Err(error) = run() {
        eprintln!("dng_gap_probe: {error}\n\n{USAGE}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let args = parse_args(env::args_os().skip(1))?;
    match args.mode {
        ProbeMode::Equality => verify_three_way_equality(&args.input)?,
        ProbeMode::Decode => {
            if !args.all_threads {
                require_single_rayon_thread()?;
            }
            warmup(args.decoder, &args.input, args.warmups)?;
            for round in 0..args.rounds {
                let measurement = decode_once(args.decoder, &args.input, args.inspect_huge_pages)?;
                println!(
                    "timing,decoder={},round={},milliseconds={:.6},anon_huge_kb={}",
                    decoder_name(args.decoder),
                    round,
                    milliseconds(measurement.elapsed),
                    measurement.anon_huge_kb
                );
            }
        }
        ProbeMode::Uncompressed => {
            require_single_rayon_thread()?;
            run_uncompressed_probe(&args)?;
        }
    }
    Ok(())
}

fn parse_args(mut args: impl Iterator<Item = std::ffi::OsString>) -> Result<ProbeArgs, String> {
    let mode = match args
        .next()
        .and_then(|arg| arg.into_string().ok())
        .as_deref()
    {
        Some("equality") => ProbeMode::Equality,
        Some("decode") => ProbeMode::Decode,
        Some("uncompressed") => ProbeMode::Uncompressed,
        Some("-h" | "--help") => return Err("help requested".to_owned()),
        Some(other) => return Err(format!("unknown mode {other:?}")),
        None => return Err("missing mode".to_owned()),
    };

    let mut decoder = ProbeDecoder::Fornax;
    let mut rounds = 1;
    let mut warmups = 1;
    let mut variant = UncompressedVariant::Fresh;
    let mut counter_only = false;
    let mut all_threads = false;
    let mut inspect_huge_pages = false;
    let mut input = None;
    while let Some(argument) = args.next() {
        let argument = argument
            .into_string()
            .map_err(|_| "arguments must be valid UTF-8".to_owned())?;
        match argument.as_str() {
            "--bench" => {}
            "--decoder" => {
                let value = args
                    .next()
                    .and_then(|value| value.into_string().ok())
                    .ok_or_else(|| "--decoder requires a value".to_owned())?;
                decoder = match value.as_str() {
                    "fornax" => ProbeDecoder::Fornax,
                    "flashraw" => ProbeDecoder::FlashRaw,
                    _ => return Err(format!("unknown decoder {value:?}")),
                };
            }
            "--rounds" => rounds = parse_count(args.next(), "--rounds")?,
            "--warmups" => warmups = parse_count(args.next(), "--warmups")?,
            "--counter-only" => counter_only = true,
            "--smaps" => inspect_huge_pages = true,
            "--threads" => {
                let value = args
                    .next()
                    .and_then(|value| value.into_string().ok())
                    .ok_or_else(|| "--threads requires a value".to_owned())?;
                all_threads = match value.as_str() {
                    "1" => false,
                    "all" => true,
                    _ => return Err(format!("unknown thread mode {value:?}")),
                };
            }
            "--variant" => {
                let value = args
                    .next()
                    .and_then(|value| value.into_string().ok())
                    .ok_or_else(|| "--variant requires a value".to_owned())?;
                variant = match value.as_str() {
                    "fresh" => UncompressedVariant::Fresh,
                    "reused" => UncompressedVariant::Reused,
                    "mmap-hugepage" => UncompressedVariant::MmapHugePage,
                    "alloc-zeroed-hugepage" => UncompressedVariant::AllocZeroedHugePage,
                    "vec-populate" => UncompressedVariant::VecPopulate,
                    "vec-hugepage" => UncompressedVariant::VecHugePage,
                    "vec-hugepage-populate" => UncompressedVariant::VecHugePagePopulate,
                    _ => return Err(format!("unknown uncompressed variant {value:?}")),
                };
            }
            value if value.starts_with('-') => {
                return Err(format!("unknown option {value:?}"));
            }
            value => {
                if input.replace(PathBuf::from(value)).is_some() {
                    return Err("only one input path is supported".to_owned());
                }
            }
        }
    }
    let input = input.ok_or_else(|| "missing input path".to_owned())?;
    if rounds == 0 {
        return Err("--rounds must be greater than zero".to_owned());
    }
    Ok(ProbeArgs {
        mode,
        decoder,
        rounds,
        warmups,
        variant,
        counter_only,
        all_threads,
        inspect_huge_pages,
        input,
    })
}

fn parse_count(value: Option<std::ffi::OsString>, option: &str) -> Result<usize, String> {
    let value = value
        .and_then(|value| value.into_string().ok())
        .ok_or_else(|| format!("{option} requires a value"))?;
    value
        .parse()
        .map_err(|error| format!("{option} value {value:?} is invalid: {error}"))
}

fn require_single_rayon_thread() -> Result<(), String> {
    match env::var("RAYON_NUM_THREADS").as_deref() {
        Ok("1") => Ok(()),
        _ => Err("set RAYON_NUM_THREADS=1 for single-thread measurements".to_owned()),
    }
}

fn warmup(decoder: ProbeDecoder, input: &Path, count: usize) -> Result<(), String> {
    for _ in 0..count {
        let _ = black_box(decode_once(decoder, input, false)?.elapsed);
    }
    Ok(())
}

struct DecodeMeasurement {
    elapsed: Duration,
    anon_huge_kb: u64,
}

fn decode_once(
    decoder: ProbeDecoder,
    input: &Path,
    inspect_huge_pages: bool,
) -> Result<DecodeMeasurement, String> {
    let start = Instant::now();
    let anon_huge_kb = match decoder {
        ProbeDecoder::Fornax => {
            let decoder =
                Decoder::decode(input).map_err(|error| format!("Fornax decode failed: {error}"))?;
            let mosaic = decoder
                .raw_mosaic()
                .map_err(|error| format!("Fornax raw_mosaic failed: {error}"))?;
            let anon_huge_kb = if inspect_huge_pages {
                mosaic_anon_huge_pages(&mosaic)?
            } else {
                0
            };
            black_box(mosaic);
            anon_huge_kb
        }
        ProbeDecoder::FlashRaw => {
            let decoder = FlashRawDecoder::decode(input)
                .map_err(|error| format!("FlashRaw decode failed: {error}"))?;
            let anon_huge_kb = if inspect_huge_pages {
                let mosaic = decoder
                    .raw_mosaic()
                    .map_err(|error| format!("FlashRaw raw_mosaic failed: {error}"))?;
                mosaic_anon_huge_pages(&mosaic)?
            } else {
                0
            };
            black_box(decoder);
            anon_huge_kb
        }
    };
    Ok(DecodeMeasurement {
        elapsed: start.elapsed(),
        anon_huge_kb,
    })
}

fn mosaic_anon_huge_pages(mosaic: &RawMosaic) -> Result<u64, String> {
    let (pointer, byte_count) = match mosaic {
        RawMosaic::U8(samples) => (
            samples.as_raw().as_ptr().cast(),
            samples.as_raw().len() * size_of::<u8>(),
        ),
        RawMosaic::U16(samples) => (
            samples.as_raw().as_ptr().cast(),
            samples.as_raw().len() * size_of::<u16>(),
        ),
        RawMosaic::U32(samples) => (
            samples.as_raw().as_ptr().cast(),
            samples.as_raw().len() * size_of::<u32>(),
        ),
        RawMosaic::F32(samples) => (
            samples.as_raw().as_ptr().cast(),
            samples.as_raw().len() * size_of::<f32>(),
        ),
    };
    smaps_anon_huge_pages(pointer, byte_count)
}

fn verify_three_way_equality(input: &Path) -> Result<(), String> {
    let source = TiffSource::from_vec(
        fs::read(input).map_err(|error| format!("reading {} failed: {error}", input.display()))?,
    );
    let dng = DngImage::parse_source(source)
        .map_err(|error| format!("parsing DNG metadata failed: {error}"))?;
    let metadata = dng
        .dng_metadata()
        .map_err(|error| format!("reading DNG metadata failed: {error}"))?;
    let bits = unsigned_scalar(metadata.get(&TiffTag::BitsPerSample.into()))
        .ok_or_else(|| "missing BitsPerSample".to_owned())?;
    let sample_format = unsigned_scalar(metadata.get(&TiffTag::SampleFormat.into())).unwrap_or(1);

    let fornax = Decoder::decode(input)
        .and_then(|decoder| decoder.raw_mosaic())
        .map_err(|error| format!("Fornax raw_mosaic failed: {error}"))?;
    let rawler = RawlerDecoder::decode(input)
        .and_then(|decoder| decoder.raw_mosaic())
        .map_err(|error| format!("Rawler raw_mosaic failed: {error}"))?;
    let flashraw = FlashRawDecoder::decode(input)
        .and_then(|decoder| decoder.raw_mosaic())
        .map_err(|error| format!("FlashRaw raw_mosaic failed: {error}"))?;

    let case = input.display().to_string();
    assert_strict_mosaic_eq(&case, "rawler", &rawler, &fornax);
    assert_strict_mosaic_eq(&case, "flashraw", &flashraw, &fornax);

    for (name, mosaic) in [
        ("fornax", &fornax),
        ("rawler", &rawler),
        ("flashraw", &flashraw),
    ] {
        let (width, height) = mosaic.dimensions();
        println!(
            "PASS,{name},{width}x{height},{},bits={bits},sample_format={sample_format}",
            storage_name(mosaic)
        );
    }
    Ok(())
}

fn assert_strict_mosaic_eq(case: &str, reference: &str, expected: &RawMosaic, actual: &RawMosaic) {
    assert_eq!(
        actual.dimensions(),
        expected.dimensions(),
        "{case}: Fornax dimensions differ from {reference}"
    );
    assert_eq!(
        storage_name(actual),
        storage_name(expected),
        "{case}: Fornax storage differs from {reference}"
    );
    match (expected, actual) {
        (RawMosaic::U8(expected), RawMosaic::U8(actual)) => {
            assert_samples_eq(case, reference, expected.as_raw(), actual.as_raw());
        }
        (RawMosaic::U16(expected), RawMosaic::U16(actual)) => {
            assert_samples_eq(case, reference, expected.as_raw(), actual.as_raw());
        }
        (RawMosaic::U32(expected), RawMosaic::U32(actual)) => {
            assert_samples_eq(case, reference, expected.as_raw(), actual.as_raw());
        }
        (RawMosaic::F32(expected), RawMosaic::F32(actual)) => {
            assert_f32_bits_eq(case, reference, expected.as_raw(), actual.as_raw());
        }
        _ => unreachable!("storage equality was checked above"),
    }
}

fn assert_samples_eq<T>(case: &str, reference: &str, expected: &[T], actual: &[T])
where
    T: Copy + Eq + std::fmt::Debug,
{
    assert_eq!(
        actual.len(),
        expected.len(),
        "{case}: Fornax sample count differs from {reference}"
    );
    if let Some(index) = expected
        .iter()
        .zip(actual)
        .position(|(expected, actual)| expected != actual)
    {
        panic!(
            "{case}: Fornax sample {index} is {:?}, {reference} sample is {:?}",
            actual[index], expected[index]
        );
    }
}

fn assert_f32_bits_eq(case: &str, reference: &str, expected: &[f32], actual: &[f32]) {
    assert_eq!(
        actual.len(),
        expected.len(),
        "{case}: Fornax sample count differs from {reference}"
    );
    if let Some(index) = expected
        .iter()
        .zip(actual)
        .position(|(expected, actual)| expected.to_bits() != actual.to_bits())
    {
        panic!(
            "{case}: Fornax sample {index} bits are {:#010x}, {reference} bits are {:#010x}",
            actual[index].to_bits(),
            expected[index].to_bits()
        );
    }
}

fn storage_name(mosaic: &RawMosaic) -> &'static str {
    match mosaic {
        RawMosaic::U8(_) => "U8",
        RawMosaic::U16(_) => "U16",
        RawMosaic::U32(_) => "U32",
        RawMosaic::F32(_) => "F32",
    }
}

fn decoder_name(decoder: ProbeDecoder) -> &'static str {
    match decoder {
        ProbeDecoder::Fornax => "fornax",
        ProbeDecoder::FlashRaw => "flashraw",
    }
}

struct UncompressedLayout {
    width: usize,
    height: usize,
    chunk_width: usize,
    chunk_height: usize,
    chunks: Vec<Chunk>,
    byte_order: ByteOrder,
}

fn run_uncompressed_probe(args: &ProbeArgs) -> Result<(), String> {
    let read_start = Instant::now();
    let bytes = fs::read(&args.input)
        .map_err(|error| format!("reading {} failed: {error}", args.input.display()))?;
    let read_elapsed = read_start.elapsed();

    let source = TiffSource::from_vec(bytes);
    let parse_start = Instant::now();
    let dng = DngImage::parse_source(source.clone())
        .map_err(|error| format!("parsing DNG failed: {error}"))?;
    let metadata = dng
        .dng_metadata()
        .map_err(|error| format!("reading DNG metadata failed: {error}"))?;
    let layout = uncompressed_layout(source.as_ref(), &metadata)?;
    let parse_elapsed = parse_start.elapsed();

    let sample_count = layout
        .width
        .checked_mul(layout.height)
        .ok_or_else(|| "sample count overflow".to_owned())?;
    let reference = if args.counter_only {
        None
    } else {
        let request = DecodeRequest {
            source: source.as_ref(),
            width: layout.width,
            height: layout.height,
            chunk_width: layout.chunk_width,
            chunk_height: layout.chunk_height,
            chunks: &layout.chunks,
            bits_per_sample: 16,
            sample_format: SampleFormat::Integer,
            byte_order: layout.byte_order,
            predictor: Predictor::None,
        };
        let reference = DecodeCompression::Uncompressed
            .decode(&request)
            .map_err(|error| format!("production uncompressed decode failed: {error}"))?
            .into_raw_u16()
            .ok_or_else(|| "production uncompressed decode did not return U16".to_owned())?;
        verify_local_variants(source.as_ref(), &layout, &reference)?;
        Some(reference)
    };

    println!(
        "setup,read_ms={:.6},parse_ms={:.6},samples={sample_count}",
        read_elapsed.as_secs_f64() * 1_000.0,
        parse_elapsed.as_secs_f64() * 1_000.0,
    );

    if !args.counter_only {
        report_uncompressed_stage_split(args, &layout, source.as_ref(), parse_elapsed)?;
    }

    let mut reused = if args.variant == UncompressedVariant::Reused {
        let mut reused = vec![0_u16; sample_count];
        decode_native_u16_into(source.as_ref(), &layout, &mut reused)?;
        reused
    } else {
        Vec::new()
    };
    for round in 0..args.rounds {
        if args.variant == UncompressedVariant::MmapHugePage {
            let allocate_start = Instant::now();
            let mut output = MmapHugePageBuffer::new(sample_count)?;
            let allocate_elapsed = allocate_start.elapsed();
            let decode_start = Instant::now();
            decode_native_u16_into(source.as_ref(), &layout, output.as_mut_slice())?;
            let decode_elapsed = decode_start.elapsed();
            if let Some(reference) = &reference
                && output.as_slice() != reference
            {
                return Err("mmap-hugepage decode differs from production".to_owned());
            }
            let huge_kb = if args.counter_only {
                0
            } else {
                smaps_anon_huge_pages(output.as_ptr(), output.byte_len())?
            };
            println!(
                "uncompressed,phase=mmap-hugepage,round={round},allocate_ms={:.6},\
                 decode_ms={:.6},anon_huge_kb={huge_kb}",
                milliseconds(allocate_elapsed),
                milliseconds(decode_elapsed),
            );
            continue;
        }

        let (mut output, allocate_elapsed) = match args.variant {
            UncompressedVariant::Fresh => {
                let allocate_start = Instant::now();
                let output = vec![0_u16; sample_count];
                (output, allocate_start.elapsed())
            }
            UncompressedVariant::Reused => {
                let output = std::mem::take(&mut reused);
                (output, Duration::ZERO)
            }
            UncompressedVariant::AllocZeroedHugePage => {
                let allocate_start = Instant::now();
                let output = allocate_zeroed_hugepage_vec(sample_count)?;
                (output, allocate_start.elapsed())
            }
            UncompressedVariant::VecPopulate
            | UncompressedVariant::VecHugePage
            | UncompressedVariant::VecHugePagePopulate => {
                let allocate_start = Instant::now();
                let output = allocate_populated_vec(
                    sample_count,
                    matches!(
                        args.variant,
                        UncompressedVariant::VecHugePage | UncompressedVariant::VecHugePagePopulate
                    ),
                    args.variant == UncompressedVariant::VecHugePagePopulate,
                )?;
                (output, allocate_start.elapsed())
            }
            UncompressedVariant::MmapHugePage => unreachable!("handled above"),
        };

        let decode_start = Instant::now();
        decode_native_u16_into(source.as_ref(), &layout, &mut output)?;
        let decode_elapsed = decode_start.elapsed();
        if let Some(reference) = &reference
            && &output != reference
        {
            return Err(format!(
                "{} decode differs from production",
                variant_name(args.variant)
            ));
        }
        let huge_kb = if args.counter_only {
            0
        } else {
            smaps_anon_huge_pages(
                output.as_ptr().cast(),
                output
                    .len()
                    .checked_mul(size_of::<u16>())
                    .ok_or_else(|| "output byte count overflow".to_owned())?,
            )?
        };
        if matches!(args.variant, UncompressedVariant::Reused) {
            reused = output;
        }
        println!(
            "uncompressed,phase={},round={round},allocate_ms={:.6},decode_ms={:.6},\
             anon_huge_kb={huge_kb}",
            variant_name(args.variant),
            allocate_elapsed.as_secs_f64() * 1_000.0,
            decode_elapsed.as_secs_f64() * 1_000.0
        );
    }
    Ok(())
}

fn report_uncompressed_stage_split(
    args: &ProbeArgs,
    layout: &UncompressedLayout,
    source: &[u8],
    parse_elapsed: Duration,
) -> Result<(), String> {
    let sample_count = layout
        .width
        .checked_mul(layout.height)
        .ok_or_else(|| "sample count overflow".to_owned())?;
    let mut reused = vec![0_u16; sample_count];

    // Warm the kernel before taking the lazy source-mapping measurements.
    decode_native_u16_into(source, layout, &mut reused)?;

    let file = File::open(&args.input)
        .map_err(|error| format!("opening {} failed: {error}", args.input.display()))?;
    let map = unsafe { Mmap::map(&file) }
        .map_err(|error| format!("mapping {} failed: {error}", args.input.display()))?;

    let source_start = Instant::now();
    touch_mapped_pages(&map)?;
    let source_faults = source_start.elapsed();

    let copy_start = Instant::now();
    decode_native_u16_into(&map, layout, &mut reused)?;
    let mapped_copy = copy_start.elapsed();

    let mut fresh_total = Duration::ZERO;
    let mut reused_total = Duration::ZERO;
    for _ in 0..args.rounds {
        let fresh_start = Instant::now();
        let mut fresh = vec![0_u16; sample_count];
        decode_native_u16_into(&map, layout, &mut fresh)?;
        fresh_total += fresh_start.elapsed();
        black_box(&fresh);

        let reused_start = Instant::now();
        decode_native_u16_into(&map, layout, &mut reused)?;
        reused_total += reused_start.elapsed();
    }
    let round_count =
        u32::try_from(args.rounds).map_err(|_| "round count does not fit in u32".to_owned())?;
    let fresh_average = fresh_total / round_count;
    let reused_average = reused_total / round_count;
    let output_first_touch = fresh_average.saturating_sub(reused_average);

    let production_start = Instant::now();
    for _ in 0..args.rounds {
        let decoder = Decoder::decode(&args.input)
            .map_err(|error| format!("Fornax decode failed: {error}"))?;
        let mosaic = decoder
            .raw_mosaic()
            .map_err(|error| format!("Fornax raw_mosaic failed: {error}"))?;
        black_box(mosaic);
    }
    let production = production_start.elapsed() / round_count;
    let sum = parse_elapsed + source_faults + output_first_touch + mapped_copy;
    let residual = production.saturating_sub(sum);

    println!(
        "stage,production_ms={:.6},parse_ms={:.6},source_mmap_faults_ms={:.6},\
         output_first_touch_ms={:.6},copy_ms={:.6},residual_ms={:.6}",
        milliseconds(production),
        milliseconds(parse_elapsed),
        milliseconds(source_faults),
        milliseconds(output_first_touch),
        milliseconds(mapped_copy),
        milliseconds(residual),
    );
    Ok(())
}

fn touch_mapped_pages(mapping: &[u8]) -> Result<(), String> {
    let page_size = probe_page_size()?;
    for page in mapping.chunks(page_size) {
        black_box(page[0]);
    }
    black_box(mapping.last());
    Ok(())
}

fn milliseconds(duration: Duration) -> f64 { duration.as_secs_f64() * 1_000.0 }

fn uncompressed_layout(
    source: &[u8],
    metadata: &fornax_tiff::Directory,
) -> Result<UncompressedLayout, String> {
    let width = required_usize(metadata, TiffTag::ImageWidth)?;
    let height = required_usize(metadata, TiffTag::ImageLength)?;
    let chunk_height = required_usize(metadata, TiffTag::RowsPerStrip)?;
    let bits = unsigned_scalar(metadata.get(&TiffTag::BitsPerSample.into()))
        .ok_or_else(|| "missing BitsPerSample".to_owned())?;
    if bits != 16 {
        return Err(format!(
            "expected 16-bit uncompressed samples, found {bits}"
        ));
    }
    let sample_format = unsigned_scalar(metadata.get(&TiffTag::SampleFormat.into())).unwrap_or(1);
    if sample_format != 1 {
        return Err(format!(
            "expected integer uncompressed samples, found sample format {sample_format}"
        ));
    }
    let offsets = unsigned_vec::<_, usize>(metadata, TiffTag::StripOffsets)
        .map_err(|error| format!("reading StripOffsets failed: {error}"))?;
    let byte_counts = unsigned_vec::<_, usize>(metadata, TiffTag::StripByteCounts)
        .map_err(|error| format!("reading StripByteCounts failed: {error}"))?;
    if offsets.len() != byte_counts.len() {
        return Err("strip offset and byte-count lists differ in length".to_owned());
    }
    let chunks = offsets
        .into_iter()
        .zip(byte_counts)
        .map(|(offset, byte_count)| Chunk { offset, byte_count })
        .collect();
    let byte_order = match source.get(..2) {
        Some(b"II") => ByteOrder::Little,
        Some(b"MM") => ByteOrder::Big,
        _ => return Err("DNG has an invalid TIFF byte-order marker".to_owned()),
    };
    Ok(UncompressedLayout {
        width,
        height,
        chunk_width: width,
        chunk_height,
        chunks,
        byte_order,
    })
}

fn required_usize(metadata: &fornax_tiff::Directory, tag: TiffTag) -> Result<usize, String> {
    unsigned_scalar(metadata.get(&tag.into()))
        .and_then(|value| usize::try_from(value).ok())
        .ok_or_else(|| format!("missing or invalid {tag}"))
}

fn verify_local_variants(
    source: &[u8],
    layout: &UncompressedLayout,
    expected: &[u16],
) -> Result<(), String> {
    let mut fresh = vec![0_u16; expected.len()];
    decode_native_u16_into(source, layout, &mut fresh)?;
    if fresh != expected {
        return Err("fresh local decode differs from production".to_owned());
    }

    let mut reused = vec![0_u16; expected.len()];
    decode_native_u16_into(source, layout, &mut reused)?;
    if reused != expected {
        return Err("reused local decode differs from production".to_owned());
    }

    let mut mmap_hugepage = MmapHugePageBuffer::new(expected.len())?;
    decode_native_u16_into(source, layout, mmap_hugepage.as_mut_slice())?;
    if mmap_hugepage.as_slice() != expected {
        return Err("mmap-hugepage local decode differs from production".to_owned());
    }
    let mmap_huge_kb = smaps_anon_huge_pages(mmap_hugepage.as_ptr(), mmap_hugepage.byte_len())?;
    println!("verify,phase=mmap-hugepage,anon_huge_kb={mmap_huge_kb}");

    let mut alloc_zeroed = allocate_zeroed_hugepage_vec(expected.len())?;
    decode_native_u16_into(source, layout, &mut alloc_zeroed)?;
    if alloc_zeroed != expected {
        return Err("alloc-zeroed-hugepage local decode differs from production".to_owned());
    }
    let alloc_zeroed_kb = smaps_anon_huge_pages(
        alloc_zeroed.as_ptr().cast(),
        expected
            .len()
            .checked_mul(size_of::<u16>())
            .ok_or_else(|| "output byte count overflow".to_owned())?,
    )?;
    println!("verify,phase=alloc-zeroed-hugepage,anon_huge_kb={alloc_zeroed_kb}");

    for (name, hugepage, populate) in [
        ("vec-populate", false, true),
        ("vec-hugepage", true, false),
        ("vec-hugepage-populate", true, true),
    ] {
        let mut output = allocate_populated_vec(expected.len(), hugepage, populate)?;
        decode_native_u16_into(source, layout, &mut output)?;
        if output != expected {
            return Err(format!("{name} local decode differs from production"));
        }
        let byte_count = output
            .len()
            .checked_mul(size_of::<u16>())
            .ok_or_else(|| "output byte count overflow".to_owned())?;
        let huge_kb = smaps_anon_huge_pages(output.as_ptr().cast(), byte_count)?;
        println!("verify,phase={name},anon_huge_kb={huge_kb}");
    }
    Ok(())
}

fn decode_native_u16_into(
    source: &[u8],
    layout: &UncompressedLayout,
    output: &mut [u16],
) -> Result<(), String> {
    let expected = layout
        .width
        .checked_mul(layout.height)
        .ok_or_else(|| "sample count overflow".to_owned())?;
    if output.len() != expected {
        return Err(format!(
            "output has {} samples, expected {expected}",
            output.len()
        ));
    }
    let across = layout.width.div_ceil(layout.chunk_width);
    for (index, chunk) in layout.chunks.iter().enumerate() {
        let tile_y_index = index / across;
        let tile_x_index = index % across;
        let tile_y = tile_y_index * layout.chunk_height;
        let tile_x = tile_x_index * layout.chunk_width;
        let copy_height = layout.chunk_height.min(layout.height - tile_y);
        let copy_width = layout.chunk_width.min(layout.width - tile_x);
        let source_end = chunk
            .offset
            .checked_add(chunk.byte_count)
            .ok_or_else(|| "strip range overflow".to_owned())?;
        let encoded = source
            .get(chunk.offset..source_end)
            .ok_or_else(|| "strip range is outside the source".to_owned())?;
        let source_stride = layout.chunk_width * 2;
        let source_row_bytes = copy_width * 2;
        let required = (copy_height - 1)
            .checked_mul(source_stride)
            .and_then(|bytes| bytes.checked_add(source_row_bytes))
            .ok_or_else(|| "strip size overflow".to_owned())?;
        if encoded.len() < required {
            return Err("strip is truncated".to_owned());
        }
        for row in 0..copy_height {
            let source_start = row * source_stride;
            let target_start = (tile_y + row) * layout.width + tile_x;
            let target = &mut output[target_start..target_start + copy_width];
            let source_row = &encoded[source_start..source_start + source_row_bytes];
            for (target, bytes) in target.iter_mut().zip(source_row.as_chunks::<2>().0) {
                *target = match layout.byte_order {
                    ByteOrder::Little => u16::from_le_bytes(*bytes),
                    ByteOrder::Big => u16::from_be_bytes(*bytes),
                };
            }
        }
    }
    Ok(())
}

fn variant_name(variant: UncompressedVariant) -> &'static str {
    match variant {
        UncompressedVariant::Fresh => "fresh",
        UncompressedVariant::Reused => "reused",
        UncompressedVariant::MmapHugePage => "mmap-hugepage",
        UncompressedVariant::AllocZeroedHugePage => "alloc-zeroed-hugepage",
        UncompressedVariant::VecPopulate => "vec-populate",
        UncompressedVariant::VecHugePage => "vec-hugepage",
        UncompressedVariant::VecHugePagePopulate => "vec-hugepage-populate",
    }
}

fn allocate_zeroed_hugepage_vec(len: usize) -> Result<Vec<u16>, String> {
    let layout = std::alloc::Layout::array::<u16>(len)
        .map_err(|error| format!("alloc_zeroed layout failed: {error}"))?;
    if layout.size() == 0 {
        return Ok(Vec::new());
    }
    let pointer = unsafe { std::alloc::alloc_zeroed(layout) }.cast::<u16>();
    let Some(pointer) = NonNull::new(pointer) else {
        return Err(format!("alloc_zeroed failed for {} bytes", layout.size()));
    };
    advise_aligned_interior(
        pointer.as_ptr().cast(),
        layout.size(),
        huge_page_size(),
        libc::MADV_HUGEPAGE,
    )?;
    // The allocation uses `layout`, is initialized to zero, and ownership
    // transfers to a Vec with the same allocator and capacity.
    Ok(unsafe { Vec::from_raw_parts(pointer.as_ptr(), len, len) })
}

fn allocate_populated_vec(len: usize, hugepage: bool, populate: bool) -> Result<Vec<u16>, String> {
    let mut output = Vec::<u16>::with_capacity(len);
    let byte_count = output
        .capacity()
        .checked_mul(size_of::<u16>())
        .ok_or_else(|| "Vec capacity byte count overflow".to_owned())?;
    let pointer = output.as_mut_ptr().cast::<u8>();

    if hugepage {
        advise_aligned_interior(pointer, byte_count, huge_page_size(), libc::MADV_HUGEPAGE)?;
    }
    if populate {
        advise_aligned_interior(
            pointer,
            byte_count,
            probe_page_size()?,
            libc::MADV_POPULATE_WRITE,
        )?;
    }

    // The advice does not initialize reused allocator memory, so keep the safe
    // resize before returning the owned Vec. Decode overwrites every value;
    // this measures the public API without exposing uninitialized samples.
    output.resize(len, 0);
    Ok(output)
}

fn advise_aligned_interior(
    pointer: *mut u8,
    byte_count: usize,
    alignment: usize,
    advice: libc::c_int,
) -> Result<usize, String> {
    let Some(start) = aligned_interior(pointer, byte_count, alignment)? else {
        return Ok(0);
    };
    let result = unsafe { libc::madvise(start.0 as *mut libc::c_void, start.1, advice) };
    if result != 0 {
        return Err(format!(
            "madvise({advice}) failed for {} bytes at {:#x}: {}",
            start.1,
            start.0,
            std::io::Error::last_os_error()
        ));
    }
    Ok(start.1)
}

fn aligned_interior(
    pointer: *mut u8,
    byte_count: usize,
    alignment: usize,
) -> Result<Option<(usize, usize)>, String> {
    if alignment == 0 || !alignment.is_power_of_two() {
        return Err(format!("invalid madvise alignment {alignment}"));
    }
    let start_address = pointer as usize;
    let end_address = start_address
        .checked_add(byte_count)
        .ok_or_else(|| "mapped address range overflow".to_owned())?;
    let start = start_address
        .checked_add(alignment - 1)
        .map(|value| value & !(alignment - 1))
        .ok_or_else(|| "aligned address overflow".to_owned())?;
    let end = end_address & !(alignment - 1);
    Ok((end > start).then_some((start, end - start)))
}

fn probe_page_size() -> Result<usize, String> {
    std::env::var("FORNAX_PROBE_PAGE_SIZE")
        .ok()
        .map(|value| {
            value
                .parse()
                .map_err(|error| format!("invalid FORNAX_PROBE_PAGE_SIZE {value:?}: {error}"))
        })
        .transpose()
        .map(|value| value.unwrap_or(4096))
}

const fn huge_page_size() -> usize { 2 << 20 }

struct MmapHugePageBuffer {
    base: NonNull<u8>,
    pointer: NonNull<u16>,
    len: usize,
    map_len: usize,
    byte_len: usize,
}

impl MmapHugePageBuffer {
    fn new(len: usize) -> Result<Self, String> {
        let byte_count = len
            .checked_mul(size_of::<u16>())
            .ok_or_else(|| "mmap-hugepage allocation size overflow".to_owned())?;
        let map_len = byte_count
            .checked_add(huge_page_size())
            .ok_or_else(|| "mmap-hugepage mapping size overflow".to_owned())?;
        let mapped = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                map_len,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
                -1,
                0,
            )
        };
        if mapped == libc::MAP_FAILED {
            return Err(format!(
                "anonymous mmap of {map_len} bytes failed: {}",
                std::io::Error::last_os_error()
            ));
        }
        let base = NonNull::new(mapped.cast::<u8>()).ok_or_else(|| {
            unsafe { libc::munmap(mapped, map_len) };
            "anonymous mmap returned null".to_owned()
        })?;
        let aligned = aligned_interior(base.as_ptr(), byte_count, huge_page_size())?
            .ok_or_else(|| "mmap-hugepage aligned interior is empty".to_owned())?;
        let pointer = NonNull::new(aligned.0 as *mut u16)
            .ok_or_else(|| "mmap-hugepage pointer is null".to_owned())?;

        // Anonymous pages are zeroed by the kernel. The range is advised
        // before any decode write, reproducing FlashRaw's ordering.
        let result = unsafe {
            libc::madvise(
                aligned.0 as *mut libc::c_void,
                aligned.1,
                libc::MADV_HUGEPAGE,
            )
        };
        if result != 0 {
            let error = std::io::Error::last_os_error();
            unsafe { libc::munmap(base.as_ptr().cast(), map_len) };
            return Err(format!("MADV_HUGEPAGE failed: {error}"));
        }
        Ok(Self {
            base,
            pointer,
            len,
            map_len,
            byte_len: byte_count,
        })
    }

    fn as_ptr(&self) -> *const u8 { self.pointer.as_ptr().cast() }

    const fn byte_len(&self) -> usize { self.byte_len }

    fn as_slice(&self) -> &[u16] {
        // Anonymous mappings are zero-initialized. The mapping remains alive
        // for this borrow.
        unsafe { std::slice::from_raw_parts(self.pointer.as_ptr(), self.len) }
    }

    fn as_mut_slice(&mut self) -> &mut [u16] {
        // The mapping contains `len` initialized u16 values and this mutable
        // borrow guarantees exclusive access.
        unsafe { std::slice::from_raw_parts_mut(self.pointer.as_ptr(), self.len) }
    }
}

impl Drop for MmapHugePageBuffer {
    fn drop(&mut self) {
        // The base pointer and length are exactly those returned by mmap.
        unsafe {
            libc::munmap(self.base.as_ptr().cast(), self.map_len);
        }
    }
}

fn smaps_anon_huge_pages(pointer: *const u8, byte_count: usize) -> Result<u64, String> {
    let start = pointer as usize;
    let end = start
        .checked_add(byte_count)
        .ok_or_else(|| "smaps range overflow".to_owned())?;
    let smaps = fs::read_to_string("/proc/self/smaps")
        .map_err(|error| format!("reading /proc/self/smaps failed: {error}"))?;
    let mut in_overlapping_mapping = false;
    let mut found_mapping = false;
    let mut huge_kb = 0_u64;
    for line in smaps.lines() {
        if let Some((mapping_start, mapping_end)) = parse_smaps_header(line) {
            in_overlapping_mapping = mapping_start < end && start < mapping_end;
            found_mapping |= in_overlapping_mapping;
            continue;
        }
        if in_overlapping_mapping && let Some(value) = line.strip_prefix("AnonHugePages:") {
            huge_kb += parse_kb(value)?;
        }
    }
    if !found_mapping {
        return Err(format!(
            "no smaps mapping contains {:#x}..{:#x}",
            start, end
        ));
    }
    Ok(huge_kb)
}

fn parse_smaps_header(line: &str) -> Option<(usize, usize)> {
    let (range, _) = line.split_once(' ')?;
    let (start, end) = range.split_once('-')?;
    Some((
        usize::from_str_radix(start, 16).ok()?,
        usize::from_str_radix(end, 16).ok()?,
    ))
}

fn parse_kb(value: &str) -> Result<u64, String> {
    value
        .split_whitespace()
        .next()
        .ok_or_else(|| "missing smaps kB value".to_owned())?
        .parse()
        .map_err(|error| format!("invalid smaps kB value {value:?}: {error}"))
}
