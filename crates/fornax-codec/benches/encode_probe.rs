#[path = "../tests/support/synthetic.rs"]
mod synthetic;

use std::env;
use std::hint::black_box;
use std::mem::size_of;
use std::ops::Range;
use std::time::{Duration, Instant};

use fornax_codec::{
    ByteOrder, EncodeCompression, EncodeRequest, ICodecEncode, Predictor, SampleBuffer,
};
use rayon::prelude::*;
use synthetic::{SyntheticPattern, uncompressed_fixture};

const USAGE: &str = "\
usage: encode_probe [--case NAME] \
[--operation encode|encode-contiguous|alloc-chunks|alloc-contiguous] \
[--warmups N] [--rounds N]";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Operation {
    Encode,
    EncodeContiguous,
    AllocateChunks,
    AllocateContiguous,
}

struct Case {
    name: &'static str,
    width: u32,
    height: u32,
    chunk_width: usize,
    chunk_height: usize,
}

const CASES: [Case; 10] = [
    Case {
        name: "5d3-u16-tile-256",
        width: 5_760,
        height: 3_840,
        chunk_width: 256,
        chunk_height: 256,
    },
    Case {
        name: "5d3-u16-strip-16",
        width: 5_760,
        height: 3_840,
        chunk_width: 5_760,
        chunk_height: 16,
    },
    Case {
        name: "small-u16-16x16",
        width: 64,
        height: 64,
        chunk_width: 16,
        chunk_height: 16,
    },
    Case {
        name: "small-smooth-u16-16x16",
        width: 64,
        height: 64,
        chunk_width: 16,
        chunk_height: 16,
    },
    Case {
        name: "medium-u16-16x16",
        width: 256,
        height: 256,
        chunk_width: 16,
        chunk_height: 16,
    },
    Case {
        name: "2m-u16-tile-256",
        width: 1_024,
        height: 1_024,
        chunk_width: 256,
        chunk_height: 256,
    },
    Case {
        name: "512k-u16-tile-256",
        width: 512,
        height: 512,
        chunk_width: 256,
        chunk_height: 256,
    },
    Case {
        name: "1m-u16-tile-256",
        width: 512,
        height: 1_024,
        chunk_width: 256,
        chunk_height: 256,
    },
    Case {
        name: "4m-u16-tile-256",
        width: 2_048,
        height: 1_024,
        chunk_width: 256,
        chunk_height: 256,
    },
    Case {
        name: "16m-u16-tile-256",
        width: 4_096,
        height: 2_048,
        chunk_width: 256,
        chunk_height: 256,
    },
];

fn main() {
    if let Err(error) = run() {
        eprintln!("encode_probe: {error}\n\n{USAGE}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let (warmups, rounds, selected_case, operation) = parse_args(env::args().skip(1))?;
    let verbose = env::var_os("ENCODE_PROBE_VERBOSE").is_some();
    for case in CASES {
        if selected_case
            .as_deref()
            .is_some_and(|selected| selected != case.name)
        {
            continue;
        }
        let pattern = if case.name.contains("smooth") {
            SyntheticPattern::Smooth
        } else {
            SyntheticPattern::Noisy { seed: 0x5d3 }
        };
        let fixture = uncompressed_fixture::<u16>(
            case.width,
            case.height,
            case.chunk_width,
            case.chunk_height,
            ByteOrder::Little,
            pattern,
        );
        for _ in 0..warmups {
            measure_operation(&fixture.image, &case, operation);
        }
        let mut samples = Vec::with_capacity(rounds);
        for round in 0..rounds {
            let elapsed = measure_operation(&fixture.image, &case, operation);
            if verbose {
                println!(
                    "encode,case={},round={},milliseconds={:.6},chunks={},input_bytes={}",
                    case.name,
                    round,
                    elapsed.as_secs_f64() * 1_000.0,
                    fixture.chunks.len(),
                    fixture.source.len(),
                );
            }
            samples.push(elapsed);
        }
        samples.sort_unstable();
        let min = samples.first().copied().unwrap_or_default();
        let middle = samples.len() / 2;
        let median = if samples.len() % 2 == 0 {
            (samples[middle - 1] + samples[middle]) / 2
        } else {
            samples[middle]
        };
        let max = samples.last().copied().unwrap_or_default();
        println!(
            "summary,case={},chunks={},bytes={},min_ms={:.6},median_ms={:.6},max_ms={:.6}",
            case.name,
            fixture.chunks.len(),
            fixture.source.len(),
            min.as_secs_f64() * 1_000.0,
            median.as_secs_f64() * 1_000.0,
            max.as_secs_f64() * 1_000.0,
        );
    }
    Ok(())
}

fn parse_args(
    mut args: impl Iterator<Item = String>,
) -> Result<(usize, usize, Option<String>, Operation), String> {
    let mut warmups = 3;
    let mut rounds = 10;
    let mut case = None;
    let mut operation = Operation::Encode;
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--bench" => {}
            "--case" => {
                case = Some(
                    args.next()
                        .ok_or_else(|| "missing value for --case".to_owned())?,
                );
            }
            "--operation" => {
                operation = match args
                    .next()
                    .ok_or_else(|| "missing value for --operation".to_owned())?
                    .as_str()
                {
                    "encode" => Operation::Encode,
                    "encode-contiguous" => Operation::EncodeContiguous,
                    "alloc-chunks" => Operation::AllocateChunks,
                    "alloc-contiguous" => Operation::AllocateContiguous,
                    other => return Err(format!("unknown operation {other:?}")),
                };
            }
            "--warmups" => {
                warmups = args
                    .next()
                    .ok_or_else(|| "missing value for --warmups".to_owned())?
                    .parse()
                    .map_err(|error| format!("invalid --warmups: {error}"))?;
            }
            "--rounds" => {
                rounds = args
                    .next()
                    .ok_or_else(|| "missing value for --rounds".to_owned())?
                    .parse()
                    .map_err(|error| format!("invalid --rounds: {error}"))?;
            }
            "-h" | "--help" => return Err("help requested".to_owned()),
            other => return Err(format!("unknown argument {other:?}")),
        }
    }
    if rounds == 0 {
        return Err("--rounds must be greater than zero".to_owned());
    }
    Ok((warmups, rounds, case, operation))
}

fn measure_operation(image: &SampleBuffer, case: &Case, operation: Operation) -> Duration {
    let start = Instant::now();
    match operation {
        Operation::Encode => {
            black_box(encode(image, case));
        }
        Operation::EncodeContiguous => {
            let encoding = encode_contiguous(image, case);
            black_box((encoding.bytes, encoding.chunks));
        }
        Operation::AllocateChunks => {
            let chunk_bytes = case.chunk_width * case.chunk_height * size_of::<u16>();
            let chunk_count = usize::try_from(image.width())
                .unwrap()
                .div_ceil(case.chunk_width)
                * usize::try_from(image.height())
                    .unwrap()
                    .div_ceil(case.chunk_height);
            black_box(
                (0..chunk_count)
                    .into_par_iter()
                    .map(|_| vec![0_u8; chunk_bytes])
                    .collect::<Vec<_>>(),
            );
        }
        Operation::AllocateContiguous => {
            let chunks = usize::try_from(image.width())
                .unwrap()
                .div_ceil(case.chunk_width)
                * usize::try_from(image.height())
                    .unwrap()
                    .div_ceil(case.chunk_height);
            let bytes = chunks * case.chunk_width * case.chunk_height * size_of::<u16>();
            black_box(vec![0_u8; bytes]);
        }
    }
    start.elapsed()
}

struct ContiguousEncoding {
    bytes: Vec<u8>,
    chunks: Vec<Range<usize>>,
}

fn encode_contiguous(image: &SampleBuffer, case: &Case) -> ContiguousEncoding {
    let SampleBuffer::U16(samples) = image else {
        panic!("the contiguous probe expects u16 samples");
    };
    let width = usize::try_from(image.width()).unwrap();
    let height = usize::try_from(image.height()).unwrap();
    let across = width.div_ceil(case.chunk_width);
    let down = height.div_ceil(case.chunk_height);
    let chunk_bytes = case.chunk_width * case.chunk_height * size_of::<u16>();
    let mut output = vec![0_u8; across * down * chunk_bytes];
    output
        .par_chunks_mut(chunk_bytes)
        .enumerate()
        .for_each(|(chunk_index, chunk)| {
            let tile_y_index = chunk_index / across;
            let tile_x_index = chunk_index % across;
            let tile_y = tile_y_index * case.chunk_height;
            let copy_height = case.chunk_height.min(height - tile_y);
            let tile_x = tile_x_index * case.chunk_width;
            let copy_width = case.chunk_width.min(width - tile_x);
            let target_row_bytes = case.chunk_width * size_of::<u16>();
            for row in 0..copy_height {
                let source_start = (tile_y + row) * width + tile_x;
                let source = &samples.as_raw()[source_start..source_start + copy_width];
                let target_start = row * target_row_bytes;
                let target_end = target_start + copy_width * size_of::<u16>();
                chunk[target_start..target_end].copy_from_slice(bytemuck::cast_slice(source));
            }
        });
    let chunks = (0..across * down)
        .map(|index| index * chunk_bytes..(index + 1) * chunk_bytes)
        .collect();
    ContiguousEncoding {
        bytes: output,
        chunks,
    }
}

fn encode(image: &SampleBuffer, case: &Case) -> usize {
    EncodeCompression::Uncompressed
        .encode(&EncodeRequest {
            image,
            chunk_width: case.chunk_width,
            chunk_height: case.chunk_height,
            byte_order: ByteOrder::Little,
            predictor: Predictor::None,
        })
        .unwrap()
        .chunks
        .iter()
        .map(Vec::len)
        .sum()
}
