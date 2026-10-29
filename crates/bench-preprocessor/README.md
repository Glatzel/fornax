# bench-preprocessor

Measures preprocessing from `DecodedRawActive` input to `RawImage` output on
deterministic synthetic Bayer images. The input image and levels are prepared
outside the timed section, and a fresh decoded input is cloned for each
iteration.

Run it from the repository root:

```bash
cargo bench -p bench-preprocessor
```

## Configuration

`FORNAX_BENCH_SIZE` sets one short edge or a comma-separated list of short
edges. Each image uses a landscape 3:2 aspect ratio, and sizes are reported in
megapixels:

```bash
FORNAX_BENCH_SIZE=1000,2000 cargo bench -p bench-preprocessor
```

The default is `1000,2000`, producing `1.50MP` and `6.00MP` images.

`FORNAX_BENCH_BITS` selects `f32`, `f64`, or both:

```bash
FORNAX_BENCH_BITS=32 cargo bench -p bench-preprocessor
```

The default is `32,64`. Preprocessing is currently single-threaded, so these
benchmarks do not sweep Rayon thread counts.

Each case name includes the stage, image size, and sample type. Criterion
accepts its normal filtering, baseline, and output arguments after `--`.
