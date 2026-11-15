mod flashraw_wrapper;
#[path = "../tests/support/mosaic_compare.rs"]
mod mosaic_compare;
mod rawler_wrapper;

use std::env;
use std::fs::{self, File};
use std::hint::black_box;
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
use mosaic_compare::{assert_mosaic_eq, canonical_mosaic};
use rawler_wrapper::RawlerDecoder;

const USAGE: &str = "\
usage:
  dng_gap_probe equality PATH
  dng_gap_probe decode --decoder fornax|flashraw --warmups N --rounds N PATH
  dng_gap_probe uncompressed --variant fresh|reused|hugepage --rounds N PATH
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
    HugePage,
}

#[derive(Debug, Eq, PartialEq)]
struct ProbeArgs {
    mode: ProbeMode,
    decoder: ProbeDecoder,
    rounds: usize,
    warmups: usize,
    variant: UncompressedVariant,
    input: PathBuf,
}

fn main() {
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
            require_single_rayon_thread()?;
            warmup(args.decoder, &args.input, args.warmups)?;
            for round in 0..args.rounds {
                let elapsed = decode_once(args.decoder, &args.input)?;
                println!(
                    "timing,decoder={},round={},milliseconds={:.6}",
                    decoder_name(args.decoder),
                    round,
                    elapsed.as_secs_f64() * 1_000.0
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
            "--variant" => {
                let value = args
                    .next()
                    .and_then(|value| value.into_string().ok())
                    .ok_or_else(|| "--variant requires a value".to_owned())?;
                variant = match value.as_str() {
                    "fresh" => UncompressedVariant::Fresh,
                    "reused" => UncompressedVariant::Reused,
                    "hugepage" => UncompressedVariant::HugePage,
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
        let _ = black_box(decode_once(decoder, input)?);
    }
    Ok(())
}

fn decode_once(decoder: ProbeDecoder, input: &Path) -> Result<Duration, String> {
    let start = Instant::now();
    match decoder {
        ProbeDecoder::Fornax => {
            let decoder =
                Decoder::decode(input).map_err(|error| format!("Fornax decode failed: {error}"))?;
            let mosaic = decoder
                .raw_mosaic()
                .map_err(|error| format!("Fornax raw_mosaic failed: {error}"))?;
            black_box(mosaic);
        }
        ProbeDecoder::FlashRaw => {
            let decoder = FlashRawDecoder::decode(input)
                .map_err(|error| format!("FlashRaw decode failed: {error}"))?;
            black_box(decoder);
        }
    }
    Ok(start.elapsed())
}

fn verify_three_way_equality(input: &Path) -> Result<(), String> {
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
    let fornax_canonical = canonical_mosaic(&fornax);
    assert_mosaic_eq(
        &case,
        "rawler",
        &canonical_mosaic(&rawler),
        &fornax_canonical,
    );
    assert_mosaic_eq(
        &case,
        "flashraw",
        &canonical_mosaic(&flashraw),
        &fornax_canonical,
    );

    for (name, mosaic) in [
        ("fornax", &fornax),
        ("rawler", &rawler),
        ("flashraw", &flashraw),
    ] {
        let (width, height) = mosaic.dimensions();
        println!("PASS,{name},{width}x{height},{}", storage_name(mosaic));
    }
    Ok(())
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

    let map_start = Instant::now();
    let file = File::open(&args.input)
        .map_err(|error| format!("opening {} failed: {error}", args.input.display()))?;
    let mmap = unsafe { Mmap::map(&file) }
        .map_err(|error| format!("mapping {} failed: {error}", args.input.display()))?;
    let map_elapsed = map_start.elapsed();
    black_box(&mmap);
    drop(mmap);

    let source = TiffSource::from_vec(bytes);
    let parse_start = Instant::now();
    let dng = DngImage::parse_source(source.clone())
        .map_err(|error| format!("parsing DNG failed: {error}"))?;
    let metadata = dng
        .dng_metadata()
        .map_err(|error| format!("reading DNG metadata failed: {error}"))?;
    let layout = uncompressed_layout(source.as_ref(), &metadata)?;
    let parse_elapsed = parse_start.elapsed();

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
    println!(
        "setup,read_ms={:.6},map_ms={:.6},parse_ms={:.6},samples={}",
        read_elapsed.as_secs_f64() * 1_000.0,
        map_elapsed.as_secs_f64() * 1_000.0,
        parse_elapsed.as_secs_f64() * 1_000.0,
        reference.len()
    );

    for round in 0..args.rounds {
        let start = Instant::now();
        let decoded = DecodeCompression::Uncompressed
            .decode(&request)
            .map_err(|error| format!("production uncompressed decode failed: {error}"))?;
        let elapsed = start.elapsed();
        black_box(decoded);
        println!(
            "uncompressed,phase=production,round={round},allocate_ms=0.000000,decode_ms={:.6}",
            elapsed.as_secs_f64() * 1_000.0
        );
    }

    let sample_count = layout
        .width
        .checked_mul(layout.height)
        .ok_or_else(|| "sample count overflow".to_owned())?;
    let mut reused = vec![0_u16; sample_count];
    decode_native_u16_into(source.as_ref(), &layout, &mut reused)?;
    for round in 0..args.rounds {
        let (allocate_elapsed, decode_elapsed) = match args.variant {
            UncompressedVariant::Fresh => {
                let allocate_start = Instant::now();
                let mut output = vec![0_u16; sample_count];
                let allocate_elapsed = allocate_start.elapsed();
                let decode_start = Instant::now();
                decode_native_u16_into(source.as_ref(), &layout, &mut output)?;
                black_box(&output);
                (allocate_elapsed, decode_start.elapsed())
            }
            UncompressedVariant::Reused => {
                let decode_start = Instant::now();
                decode_native_u16_into(source.as_ref(), &layout, &mut reused)?;
                black_box(&reused);
                (Duration::ZERO, decode_start.elapsed())
            }
            UncompressedVariant::HugePage => {
                let allocate_start = Instant::now();
                let mut output = HugePageBuffer::new(sample_count)?;
                let allocate_elapsed = allocate_start.elapsed();
                let decode_start = Instant::now();
                decode_native_u16_into(source.as_ref(), &layout, output.as_mut_slice())?;
                black_box(output.as_slice());
                (allocate_elapsed, decode_start.elapsed())
            }
        };
        println!(
            "uncompressed,phase={},round={round},allocate_ms={:.6},decode_ms={:.6}",
            variant_name(args.variant),
            allocate_elapsed.as_secs_f64() * 1_000.0,
            decode_elapsed.as_secs_f64() * 1_000.0
        );
    }
    Ok(())
}

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

    let mut hugepage = HugePageBuffer::new(expected.len())?;
    decode_native_u16_into(source, layout, hugepage.as_mut_slice())?;
    if hugepage.as_slice() != expected {
        return Err("hugepage local decode differs from production".to_owned());
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
        UncompressedVariant::HugePage => "hugepage",
    }
}

struct HugePageBuffer {
    pointer: NonNull<u16>,
    len: usize,
    layout: std::alloc::Layout,
}

impl HugePageBuffer {
    fn new(len: usize) -> Result<Self, String> {
        let byte_count = len
            .checked_mul(size_of::<u16>())
            .ok_or_else(|| "hugepage allocation size overflow".to_owned())?;
        let page_size = std::env::var("FORNAX_PROBE_PAGE_SIZE")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(4096_usize);
        let layout = std::alloc::Layout::from_size_align(byte_count, page_size)
            .map_err(|error| format!("hugepage layout failed: {error}"))?;
        // The layout is non-zero, the returned allocation uses this exact
        // layout, and every typed sample is initialized by alloc_zeroed.
        let pointer = unsafe { std::alloc::alloc_zeroed(layout) };
        let pointer = NonNull::new(pointer.cast::<u16>())
            .ok_or_else(|| "alloc_zeroed returned null".to_owned())?;
        #[cfg(target_os = "linux")]
        {
            // The layout is page-aligned, so the advised range satisfies
            // madvise's address and length requirements.
            let result = unsafe {
                libc::madvise(pointer.as_ptr().cast(), layout.size(), libc::MADV_HUGEPAGE)
            };
            if result != 0 {
                let error = std::io::Error::last_os_error();
                unsafe { std::alloc::dealloc(pointer.as_ptr().cast(), layout) };
                return Err(format!("MADV_HUGEPAGE failed: {error}"));
            }
        }
        Ok(Self {
            pointer,
            len,
            layout,
        })
    }

    fn as_slice(&self) -> &[u16] {
        // The allocation contains `len` initialized u16 values and remains
        // alive for the returned borrow.
        unsafe { std::slice::from_raw_parts(self.pointer.as_ptr(), self.len) }
    }

    fn as_mut_slice(&mut self) -> &mut [u16] {
        // The allocation contains `len` initialized u16 values and this
        // mutable borrow guarantees exclusive access.
        unsafe { std::slice::from_raw_parts_mut(self.pointer.as_ptr(), self.len) }
    }
}

impl Drop for HugePageBuffer {
    fn drop(&mut self) {
        // The pointer was allocated with this exact layout.
        unsafe { std::alloc::dealloc(self.pointer.as_ptr().cast(), self.layout) };
    }
}
