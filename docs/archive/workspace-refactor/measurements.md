# Workspace refactor measurements

This archived workspace-refactor record documents Workspace refactor measurements.

## Environment and method

- Machine: AMD Ryzen 7 5800H, 8 cores and 15 online logical CPUs, WSL2 on
  Linux 6.6.87.1.
- Toolchain: Cargo 1.99.1, 2026-08-27.
- Baseline: `9b9c1b284ff5f4fe9ef1c8ff41eb6c3cff17b8d2`.
- Current main: `8b0e826df427ce58831054b45039cf279c5af52b`.
- Temporary detached worktrees were created outside the repository.
- Benchmarks used one Rayon thread, 32-bit samples, a 256-pixel short edge,
  all 11 demosaic algorithms, Criterion sample size 10, 0.2 seconds warm-up,
  and 1 second measurement.
- Criterion intervals below are its low estimate, median estimate, and high
  estimate. The interval is reported as the spread.

Benchmark commands:

```bash
RAYON_NUM_THREADS=1 FORNAX_BENCH_SIZE=256 FORNAX_BENCH_BITS=32 \
FORNAX_BENCH_ALGORITHM=aahd,ahd,amaze,dcb,dht,linear,lmmse,ppg,rcd,vng,vng4 \
cargo bench -p fornax-dng --all-features --bench writer -- \
  --noplot --sample-size 10 --warm-up-time 0.2 --measurement-time 1

RAYON_NUM_THREADS=1 FORNAX_BENCH_SIZE=256 FORNAX_BENCH_BITS=32 \
FORNAX_BENCH_ALGORITHM=aahd,ahd,amaze,dcb,dht,linear,lmmse,ppg,rcd,vng,vng4 \
cargo bench -p fornax-postprocessor --bench demosaic -- \
  --noplot --sample-size 10 --warm-up-time 0.2 --measurement-time 1

RAYON_NUM_THREADS=1 FORNAX_BENCH_SIZE=256 FORNAX_BENCH_BITS=32 \
cargo bench -p fornax-preprocessor --bench preprocessor -- \
  --noplot --sample-size 10 --warm-up-time 0.2 --measurement-time 1

RAYON_NUM_THREADS=1 FORNAX_BENCH_SIZE=256 FORNAX_BENCH_BITS=32 \
FORNAX_BENCH_ALGORITHM=aahd,ahd,amaze,dcb,dht,linear,lmmse,ppg,rcd,vng,vng4 \
cargo bench -p fornax-raw-pipeline --bench demosaic -- \
  --noplot --sample-size 10 --warm-up-time 0.2 --measurement-time 1

RAYON_NUM_THREADS=1 FORNAX_BENCH_SIZE=256 FORNAX_BENCH_BITS=32 \
cargo bench -p fornax-raw-pipeline --bench preprocessor -- \
  --noplot --sample-size 10 --warm-up-time 0.2 --measurement-time 1
```

The baseline command used `fornax-postprocessor` and `fornax-preprocessor`;
current main uses their moved `fornax-raw-pipeline` targets. The benchmark
source and case names are otherwise the same.

## DNG writer benchmark

| Case | Baseline low / median / high | Main low / median / high |
| --- | --- | --- |
| uncompressed | 10.798 / 10.931 / 11.145 us | 15.912 / 19.362 / 21.212 us |
| lossless-jpeg | 1.3811 / 1.3888 / 1.3952 ms | 1.4104 / 1.4131 / 1.4165 ms |
| deflate | 1.0310 / 1.0376 / 1.0444 ms | 1.0291 / 1.0338 / 1.0380 ms |
| zstd | 377.55 / 379.46 / 380.72 us | 280.83 / 282.29 / 283.89 us |

These are writer timings. No `fornax-codec` benchmark target exists at either
revision, and `fornax-dng` has no decode benchmark target. Its optional real
fixture is parsed and decoded only while constructing benchmark input. This
run used the deterministic synthetic noise input, so no decode operation was
timed.

## Stage benchmarks

| Case | Baseline low / median / high | Main low / median / high |
| --- | --- | --- |
| preprocess/0.10MP/f32 | 63.546 / 66.050 / 68.676 us | 64.483 / 66.659 / 69.443 us |
| aahd/0.10MP/f32 | 22.374 / 22.442 / 22.508 ms | 22.217 / 22.388 / 22.663 ms |
| ahd/0.10MP/f32 | 22.590 / 22.690 / 22.851 ms | 22.887 / 23.036 / 23.134 ms |
| amaze/0.10MP/f32 | 21.757 / 21.805 / 21.852 ms | 21.690 / 21.853 / 22.002 ms |
| dcb/0.10MP/f32 | 17.350 / 17.434 / 17.493 ms | 17.312 / 17.343 / 17.372 ms |
| dht/0.10MP/f32 | 8.5857 / 8.6220 / 8.6410 ms | 8.6077 / 8.6807 / 8.7535 ms |
| linear/0.10MP/f32 | 115.81 / 116.67 / 117.25 us | 114.94 / 115.36 / 115.80 us |
| lmmse/0.10MP/f32 | 28.264 / 28.338 / 28.421 ms | 28.491 / 28.622 / 28.748 ms |
| ppg/0.10MP/f32 | 7.2283 / 7.2528 / 7.2853 ms | 7.2391 / 7.3021 / 7.3545 ms |
| rcd/0.10MP/f32 | 10.065 / 10.085 / 10.109 ms | 10.143 / 10.177 / 10.230 ms |
| vng/0.10MP/f32 | 18.668 / 18.708 / 18.769 ms | 18.467 / 18.509 / 18.553 ms |
| vng4/0.10MP/f32 | 18.742 / 18.941 / 19.120 ms | 18.475 / 18.537 / 18.590 ms |

## Build timings at main

Each clean run used a new `CARGO_TARGET_DIR`. The incremental run touched the
package's `src/lib.rs` and rebuilt in the same target directory.

```bash
CARGO_TARGET_DIR=<clean-target> cargo build --timings -p fornax-codec
touch crates/fornax-codec/src/lib.rs
CARGO_TARGET_DIR=<clean-target> cargo build --timings -p fornax-codec

CARGO_TARGET_DIR=<clean-target> cargo build --timings -p fornax-tiff
touch crates/fornax-tiff/src/lib.rs
CARGO_TARGET_DIR=<clean-target> cargo build --timings -p fornax-tiff
```

| Package | Clean wall time | Incremental wall time |
| --- | --- | --- |
| fornax-codec | 6.04 s | 0.30 s |
| fornax-tiff | 6.86 s | 0.30 s |

## Golden hashes

```bash
pixi run -e dev cargo run -q -p decoder-tracker -- hash \
  --manifest crates/decoder-tracker/golden/selected-manifest.csv \
  --check crates/decoder-tracker/golden/decode-hashes.csv
```

Result: `golden: unchanged (9 rows)`.
