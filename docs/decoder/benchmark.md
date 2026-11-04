# Decoder benchmark

## Current benchmark

`crates/fornax-core/benches/decoder.rs` measures full DNG decoding through
`fornax_core::Decoder`:

- file read and container parsing;
- raw-IFD selection and metadata extraction;
- strip/tile payload decode;
- `MosaicData` construction.

The default fixture is
`external/raw-images/images/colorchart-5D2-6000K.dng`. Override it with:

```bash
FORNAX_BENCH_RAW=path/to/input.dng cargo bench -p fornax-core
```

The current benchmark groups are `full/f32` and `full/f64`, which exercise the
pipeline metadata conversion without adding preprocess, demosaic, Python
conversion, preview decode, or DNG writing to the timed region.

Run it from the repository root:

```bash
cargo bench -p fornax-core
```

Compile-only validation is useful when the fixture is unavailable:

```bash
cargo bench -p fornax-core --no-run
```

## Comparison extension

The open decoder benchmark task extends this harness to a small pinned corpus
set and can use Rawler only as a development dependency. Rawler must not enter
the normal dependency trees of `fornax-core`, `fornax-preprocessor`,
`fornax-postprocessor`, or `fornax`.

The extension should:

1. Resolve files through the corpus manifest rather than committed RAW files.
2. Run native and reference decoders in the same process on the same input.
3. Exclude sample-type conversion from the decode-only region unless a second
   converted-output measurement is reported separately.
4. Report each file and a geometric mean without hiding a large single-file
   regression.
5. Record host class, corpus revision, and command in review evidence; do not
   commit host-dependent timing baselines.

The diagnostic target is a Fornax/Rawler median ratio between 0.80 and 1.25
once enough files are stable. A large ratio is investigated, not papered over
with an aggregate.

## Definition of done for the extension

- The benchmark builds with Rawler only as a dev dependency.
- The fixed set resolves only through the corpus manifest.
- The report separates read/parse/decode from sample conversion.
- Dependency-tree checks prove Rawler is absent from production crates.
- Single-file regressions remain visible in review output.
