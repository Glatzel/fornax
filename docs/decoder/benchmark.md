# Decoder benchmark

## Current benchmark

`fornax-decoder` is the workspace benchmark package for decoder comparisons. Its
`decoder` target measures the default decoder in `fornax-decoder` (DNG and
supported Sony ARW/NRW) against the Rawler adapter used by the benchmark.

For each selected decoder, the target benchmarks every method in
`fornax_raw_core::protocol::IDecoder`:

- `decode`;
- `raw_mosaic`;
- `cropped_mosaic`;
- `dng_metadata`;
- `process_metadata`.

The instance-method benchmarks construct one decoder outside the timed region,
so they measure the requested accessor rather than reparsing the file. The
`decode` benchmark includes file read and decoder construction.

`f32` is the default sample type. Select `f64` or both with
`FORNAX_BENCH_BITS=64` or `FORNAX_BENCH_BITS=32,64`.
`FORNAX_BENCH_DECODER` selects `fornax`, `rawler`, or a comma-separated list;
the default is both.

The default fixture is
`external/raw-images/images/colorchart-5D2-6000K.dng`. Override it with:

```bash
FORNAX_BENCH_RAW=path/to/input.dng cargo bench -p fornax-decoder
```

Each decoder prints its metadata tag count before the metadata timings.
Compare those timings only when both decoders expose equivalent metadata.

Run it from the repository root:

```bash
cargo bench -p fornax-decoder
```

Compile-only validation is useful when the fixture is unavailable or benchmark
setup is changing:

```bash
cargo bench -p fornax-decoder --no-run
```

The CI benchmark matrix invokes `-p fornax-decoder`, whose `decoder` benchmark
target owns decoder comparisons.

## Fixed-corpus comparison

The open `[DEC-BENCH]` item extends the current fixture benchmark into a
fixed-set, Fornax-versus-Rawler decode comparison. This extension is not
implemented yet.

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
once enough files are stable. A large ratio is investigated, not hidden with an
aggregate.

## Definition of done for the extension

- The benchmark resolves its fixed set only through the corpus manifest.
- The report separates file read/parse/decode from sample conversion.
- Every input and decoder method remains visible in single-file output.
- Dependency-tree checks prove comparison-only references stay out of
  production decoder crates.
- Single-file regressions remain visible in review output.
