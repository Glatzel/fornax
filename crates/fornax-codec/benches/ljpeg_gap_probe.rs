use std::env;
use std::hint::black_box;
use std::time::Instant;

use fornax_codec::ljpeg::{decode_interleaved, encode_interleaved_for_test};

const WIDTH: u16 = 256;
const HEIGHT: u16 = 256;
const PRECISION: u8 = 14;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ScanPattern {
    Smooth,
    Noisy,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ScanSpec {
    components: u8,
    pattern: ScanPattern,
    tiles: usize,
}

struct SyntheticScan {
    encoded: Vec<Vec<u8>>,
    samples: Vec<u16>,
    output_width: usize,
}

#[derive(Debug)]
struct ProbeArgs {
    components: Vec<u8>,
    patterns: Vec<ScanPattern>,
    tiles: Vec<usize>,
    rounds: usize,
    self_check: bool,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("ljpeg_gap_probe: {error}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let args = parse_args(env::args_os().skip(1))?;
    if args.self_check {
        self_check()?;
        return Ok(());
    }
    for components in args.components {
        for pattern in &args.patterns {
            for tiles in &args.tiles {
                let spec = ScanSpec {
                    components,
                    pattern: *pattern,
                    tiles: *tiles,
                };
                let scan = synthetic_scan(&spec)?;
                verify_scan(&spec, &scan)?;
                for round in 0..args.rounds {
                    let start = Instant::now();
                    for encoded in &scan.encoded {
                        let decoded = decode_interleaved(black_box(encoded), PRECISION)
                            .map_err(|error| format!("decoding {spec:?} failed: {error}"))?;
                        black_box(decoded.samples);
                    }
                    println!(
                        "benchmark,components={},pattern={},tiles={},round={},milliseconds={:.6}",
                        spec.components,
                        pattern_name(spec.pattern),
                        spec.tiles,
                        round,
                        start.elapsed().as_secs_f64() * 1_000.0
                    );
                }
            }
        }
    }
    Ok(())
}

fn parse_args(mut args: impl Iterator<Item = std::ffi::OsString>) -> Result<ProbeArgs, String> {
    let mut components = vec![1, 2];
    let mut patterns = vec![ScanPattern::Smooth, ScanPattern::Noisy];
    let mut tiles = vec![1, 64, 256];
    let mut rounds = 5;
    let mut self_check = false;
    while let Some(argument) = args.next() {
        let argument = argument
            .into_string()
            .map_err(|_| "arguments must be valid UTF-8".to_owned())?;
        match argument.as_str() {
            "--bench" => {}
            "--self-check" => self_check = true,
            "--components" => {
                components = parse_list(args.next(), "--components", |value| match value {
                    "1" => Some(1),
                    "2" => Some(2),
                    _ => None,
                })?;
            }
            "--pattern" => {
                patterns = parse_list(args.next(), "--pattern", |value| match value {
                    "smooth" => Some(ScanPattern::Smooth),
                    "noisy" => Some(ScanPattern::Noisy),
                    _ => None,
                })?;
            }
            "--tiles" => {
                tiles = parse_list(args.next(), "--tiles", |value| value.parse().ok())?;
            }
            "--rounds" => {
                let value = args
                    .next()
                    .and_then(|value| value.into_string().ok())
                    .ok_or_else(|| "--rounds requires a value".to_owned())?;
                rounds = value
                    .parse()
                    .map_err(|error| format!("invalid --rounds value {value:?}: {error}"))?;
            }
            value => return Err(format!("unknown argument {value:?}")),
        }
    }
    if components.is_empty() || patterns.is_empty() || tiles.is_empty() || rounds == 0 {
        return Err("components, patterns, tiles, and rounds must be nonempty".to_owned());
    }
    Ok(ProbeArgs {
        components,
        patterns,
        tiles,
        rounds,
        self_check,
    })
}

fn parse_list<T>(
    value: Option<std::ffi::OsString>,
    option: &str,
    parse: impl Fn(&str) -> Option<T>,
) -> Result<Vec<T>, String> {
    let value = value
        .and_then(|value| value.into_string().ok())
        .ok_or_else(|| format!("{option} requires a value"))?;
    value
        .split(',')
        .map(str::trim)
        .map(|value| parse(value).ok_or_else(|| format!("invalid {option} value {value:?}")))
        .collect()
}

fn self_check() -> Result<(), String> {
    for components in [1, 2] {
        for pattern in [ScanPattern::Smooth, ScanPattern::Noisy] {
            for tiles in [1, 64, 256] {
                let spec = ScanSpec {
                    components,
                    pattern,
                    tiles,
                };
                let first = synthetic_scan(&spec)?;
                let second = synthetic_scan(&spec)?;
                if first.encoded != second.encoded || first.samples != second.samples {
                    return Err(format!("{spec:?} generator is not deterministic"));
                }
                verify_scan(&spec, &first)?;
                if pattern == ScanPattern::Noisy
                    && !first
                        .encoded
                        .iter()
                        .all(|tile| tile.windows(2).any(|bytes| bytes == [0xff, 0x00]))
                {
                    return Err(format!("{spec:?} does not exercise byte stuffing"));
                }
                println!(
                    "PASS,components={components},pattern={},tiles={tiles},bytes={}",
                    pattern_name(pattern),
                    first.encoded.iter().map(Vec::len).sum::<usize>()
                );
            }
        }
    }
    Ok(())
}

fn verify_scan(spec: &ScanSpec, scan: &SyntheticScan) -> Result<(), String> {
    if scan.encoded.len() != spec.tiles {
        return Err(format!("{spec:?} produced {} tiles", scan.encoded.len()));
    }
    for encoded in &scan.encoded {
        let decoded = decode_interleaved(encoded, PRECISION)
            .map_err(|error| format!("decoding {spec:?} failed: {error}"))?;
        if decoded.width != scan.output_width
            || decoded.height != usize::from(HEIGHT)
            || decoded.components != usize::from(spec.components)
            || decoded.samples != scan.samples
        {
            return Err(format!("{spec:?} changed dimensions or sample bits"));
        }
    }
    Ok(())
}

fn synthetic_scan(spec: &ScanSpec) -> Result<SyntheticScan, String> {
    if !matches!(spec.components, 1 | 2) {
        return Err(format!("unsupported component count {}", spec.components));
    }
    let sample_count = usize::from(WIDTH)
        .checked_mul(usize::from(HEIGHT))
        .and_then(|count| count.checked_mul(usize::from(spec.components)))
        .ok_or_else(|| "synthetic sample count overflow".to_owned())?;
    let samples = generate_samples(spec, sample_count);
    let encoded = encode_interleaved_for_test(&samples, WIDTH, HEIGHT, spec.components, PRECISION);
    Ok(SyntheticScan {
        encoded: vec![encoded; spec.tiles],
        samples,
        output_width: usize::from(WIDTH) * usize::from(spec.components),
    })
}

fn generate_samples(spec: &ScanSpec, count: usize) -> Vec<u16> {
    let mask = (1_u16 << PRECISION) - 1;
    let mut state = 0x9e37_79b9_7f4a_7c15_u64
        ^ u64::from(spec.components)
        ^ u64::from(matches!(spec.pattern, ScanPattern::Noisy));
    let mut smooth = vec![1_u16 << (PRECISION - 1); usize::from(spec.components)];
    (0..count)
        .map(|index| match spec.pattern {
            ScanPattern::Smooth => {
                state = xorshift(state);
                let delta = match state % 3 {
                    0 => 0_u16,
                    1 => 1_u16,
                    _ => mask,
                };
                let component = index % usize::from(spec.components);
                smooth[component] = smooth[component].wrapping_add(delta) & mask;
                smooth[component]
            }
            ScanPattern::Noisy => {
                state = xorshift(state);
                (state as u16) & mask
            }
        })
        .collect()
}

const fn xorshift(mut state: u64) -> u64 {
    state ^= state << 13;
    state ^= state >> 7;
    state ^= state << 17;
    state
}

const fn pattern_name(pattern: ScanPattern) -> &'static str {
    match pattern {
        ScanPattern::Smooth => "smooth",
        ScanPattern::Noisy => "noisy",
    }
}
