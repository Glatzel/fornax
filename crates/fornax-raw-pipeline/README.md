# fornax-raw-pipeline

Owns the stages after decoding and before output encoding: black/white-level
normalization, Bayer demosaicing, RGB reconstruction, and optional color-space
processing.

```rust
use fornax_raw_core::protocol::{IPostProcessor, IPreprocessor};
use fornax_raw_pipeline::{OutputColorSpace, PostProcessor, Preprocessor};

let normalized = Preprocessor.process(decoded_active, &metadata.levels)?;
let processor = PostProcessor {
    output_color: OutputColorSpace::Srgb,
    ..PostProcessor::default()
};
let rgb = processor.process(normalized.image(), &metadata)?;
```

## Demosaicing

The `Demosaic` selector supports linear, PPG, RCD, AHD, VNG, four-color VNG4,
DCB, DHT, AAHD, AMaZE, and LMMSE reconstruction for conventional 2x2 Bayer
images. `IDemosaic`, `LazyDemosaic`, and `DemosaicError` remain available
through the same public names used by the former stage crates.

All eleven demosaic variants are implemented in Rust. LibRaw and darktable are
used only by the separate comparison crate as behavioral references. Every
demosaic accepts and returns normalized scene-linear data. AAHD gamma-encodes
samples only for homogeneity scoring, while LMMSE performs interpolation in a
gamma-encoded workspace and decodes the result before returning.

DCB and LMMSE parameters are carried directly by the `Demosaic` enum:

```rust
use fornax_raw_pipeline::{Demosaic, LmmseMode};

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

## Color processing

`PostProcessor::output_color` defaults to `OutputColorSpace::Raw`, which leaves
demosaiced camera RGB unchanged apart from normalized white balance. Setting it
to `Srgb`, `AdobeRgb`, `WideGamutRgb`, `ProPhotoRgb`, `Xyz`, `Aces2065_1`,
`AcesCg`, `DciP3`, or `Rec2020` applies a camera-to-XYZ matrix and then the
selected XYZ-to-output matrix after white balance in the same output-row pass.

DNG `ForwardMatrix1` is preferred, followed by `ForwardMatrix2`. If neither is
available, `ColorMatrix1` or `ColorMatrix2` is combined with its matching
`CameraCalibration` and inverted. The conversion is scene-linear; it does not
apply a display transfer function.

## Benchmarks

Preprocessing benchmarks run with:

```bash
cargo bench -p fornax-raw-pipeline --bench preprocessor
```

Demosaic benchmarks run with:

```bash
cargo bench -p fornax-raw-pipeline --bench demosaic
```

`FORNAX_BENCH_SIZE` sets one short edge or a comma-separated list of short
edges. Each image uses a landscape 3:2 aspect ratio, and sizes are reported in
megapixels. The default is `1000,2000`, producing `1.50MP` and `6.00MP` images.

`FORNAX_BENCH_BITS` selects `f32`, `f64`, or both. The default is `32`.
Preprocessing is currently single-threaded, so those benchmarks do not sweep
Rayon thread counts.

`FORNAX_BENCH_ALGORITHM` selects one demosaic algorithm by ID, such as `ahd`,
`dcb`, or `lmmse`. An unset or empty value runs every registered algorithm.
Demosaic benchmarks also sweep Rayon thread counts; set `RAYON_NUM_THREADS` to
control the maximum sweep.
