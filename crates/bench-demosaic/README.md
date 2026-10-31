# bench-demosaic

Measures the demosaic stage on deterministic synthetic Bayer images. The
benchmark currently exercises each case registered by the bench binary and
collects the complete RGB output so row-based algorithms perform their work
inside the timed section.

Run it from the repository root:

```bash
cargo bench -p bench-demosaic
```

## Configuration

`FORNAX_BENCH_SIZE` sets one short edge or a comma-separated list of short
edges. Each image uses a landscape 3:2 aspect ratio, and sizes are reported in
megapixels:

```bash
FORNAX_BENCH_SIZE=1000,2000 cargo bench -p bench-demosaic
```

The default is `1000,2000`, producing `1.50MP` and `6.00MP` images.

`FORNAX_BENCH_BITS` selects `f32`, `f64`, or both:

```bash
FORNAX_BENCH_BITS=32 cargo bench -p bench-demosaic
```

The default is `32,64`.

`FORNAX_BENCH_ALGORITHM` selects one algorithm from
`crates/bench-demosaic/algorithms.json`. The value is an ID such as `ahd`,
`dcb`, or `lmmse`; an unset or empty value runs every registered algorithm:

```bash
FORNAX_BENCH_ALGORITHM=ahd cargo bench -p bench-demosaic
```

CI generates one simulation job per registry entry with
`scripts/benchmark-matrix.py`. Add new benchmarkable algorithms to the JSON
registry and map the ID to its `Demosaic` variant in
`crates/bench-demosaic/src/lib.rs`.

Demosaic benchmarks also sweep Rayon thread counts. Set
`RAYON_NUM_THREADS` to control the maximum sweep:

```bash
RAYON_NUM_THREADS=4 cargo bench -p bench-demosaic
```

The sweep contains powers of two from one through the available pool size.
When more than one thread count is measured, names include a suffix such as
`/4T`; a single-thread-count run omits the suffix to preserve baseline names.
When `CI` is set, only one thread is used so simulation benchmarks remain
stable.

Each case name includes the algorithm, image size, sample type, and, when
applicable, thread count. Criterion accepts its normal filtering, baseline,
and output arguments after `--`.
