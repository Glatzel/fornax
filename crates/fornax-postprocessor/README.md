# fornax-postprocessor

Post-processes normalized Bayer data into RGB output. The `Demosaic` selector
supports linear, PPG, RCD, AHD, VNG, four-color VNG4, DCB, DHT, AAHD, AMaZE,
and LMMSE reconstruction for conventional 2x2 Bayer images.

The crate owns the `IDemosaic` and `LazyDemosaic` protocol traits in
`fornax_postprocessor::protocol`. `DemosaicError` remains in
`fornax_core::error`.

All eleven demosaic variants are implemented in Rust. LibRaw and darktable are
used only by the separate comparison crate as behavioral references. Every
demosaic accepts and returns normalized scene-linear data. AAHD gamma-encodes
samples only for homogeneity scoring, while LMMSE performs interpolation in a
gamma-encoded workspace and decodes the result before returning.

DCB and LMMSE parameters are carried directly by the `Demosaic` enum:

```rust
use fornax_postprocessor::{Demosaic, LmmseMode};

let dcb = Demosaic::Dcb {
    iterations: 1,
    enhance: true,
};
let lmmse = Demosaic::Lmmse {
    mode: LmmseMode::ThreeMedians,
};
assert!(matches!(dcb, Demosaic::Dcb { iterations: 1, .. }));
assert!(matches!(lmmse, Demosaic::Lmmse { .. }));
```

`Demosaic::dcb()` preserves the comparison default `(0, true)`.
`Demosaic::lmmse()` preserves darktable mode 1: one median pass and no
refinement.

```rust
use fornax_core::protocol::IPostProcessor;
use fornax_postprocessor::PostProcessor;

let processor = PostProcessor::default();
let rgb = processor.process(&raw_image, &metadata)?;
```

## Demosaic benchmarks

Measures the demosaic stage on deterministic synthetic Bayer images. The
benchmark exercises each case registered by the bench binary and collects the
complete RGB output so row-based algorithms perform their work inside the
timed section.

Run it from the repository root:

```bash
cargo bench -p fornax-postprocessor
```

`FORNAX_BENCH_SIZE` sets one short edge or a comma-separated list of short
edges. Each image uses a landscape 3:2 aspect ratio, and sizes are reported in
megapixels:

```bash
FORNAX_BENCH_SIZE=1000,2000 cargo bench -p fornax-postprocessor
```

The default is `1000,2000`, producing `1.50MP` and `6.00MP` images.

`FORNAX_BENCH_BITS` selects `f32`, `f64`, or both:

```bash
FORNAX_BENCH_BITS=32 cargo bench -p fornax-postprocessor
```

The default is `32,64`.

`FORNAX_BENCH_ALGORITHM` selects one algorithm from
`crates/fornax-postprocessor/benches/algorithms.json`. The value is an ID such
as `ahd`, `dcb`, or `lmmse`; an unset or empty value runs every registered
algorithm:

```bash
FORNAX_BENCH_ALGORITHM=ahd cargo bench -p fornax-postprocessor
```

Add new benchmarkable algorithms to the JSON registry and map the ID to its
`Demosaic` variant in
`crates/fornax-postprocessor/benches/support/demosaic_registry.rs`.

Demosaic benchmarks also sweep Rayon thread counts. Set
`RAYON_NUM_THREADS` to control the maximum sweep:

```bash
RAYON_NUM_THREADS=4 cargo bench -p fornax-postprocessor
```

The sweep contains powers of two from one through the available pool size.
When more than one thread count is measured, names include a suffix such as
`/4T`; a single-thread-count run omits the suffix to preserve baseline names.
When `CI` is set, only one thread is used so simulation benchmarks remain
stable.

Each case name includes the algorithm, image size, sample type, and, when
applicable, thread count. Criterion accepts its normal filtering, baseline,
and output arguments after `--`.
