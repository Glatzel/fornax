# Decoder Benchmarks

This guide defines the golden-corpus decoder benchmark, its comparison scope, and its CI matrix.

## Current benchmark

`fornax-decoder` owns the `decoder` benchmark target. It compares the default
pure-Rust decoder with benchmark-only Rawler and FlashRaw adapters on every row
of `crates/decoder-tracker/golden/selected-manifest.csv`.

The golden set contains 30 known-passing conventional 2x2 RGB Bayer files
across DNG, Canon CR3, Sony ARW, Nikon NEF, Fujifilm RAF, Hasselblad 3FR,
Panasonic RW2, and Phase One/Leaf IIQ. The 3FR rows cover both accepted payload
families: uncompressed compression 1 and predictor-8 compression 7.

For every input, the benchmark creates one Criterion group containing:

- Fornax `end_to_end`, measured as file read, container parse, sensor decode,
  transforms, and native raw-mosaic allocation;
- Fornax `decode`, measured as file read, container parse, and decoder
  construction;
- Fornax `raw_mosaic`, measured on one decoder constructed outside the timed
  region;
- Fornax `cropped_mosaic`, measured with `f32` conversion and active-area crop;
- Rawler and FlashRaw `decode`, with raw-mosaic support checked only during
  preflight.

Fornax also reports `dng_metadata` and `process_metadata` as diagnostics.
References do not run `cropped_mosaic`, excluding active-area conversion work
and avoiding missing-active-area panics. FlashRaw has no equivalent TIFF
`Directory` API and the Rawler benchmark adapter does not expose normalized
process metadata, so those accessors are not compared across implementations.

## Input selection

The benchmark defaults to:

```text
crates/decoder-tracker/golden/selected-manifest.csv
external/raw.pixls.us
```

Override the manifest or corpus root with `FORNAX_BENCH_MANIFEST` and
`FORNAX_BENCH_CORPUS`. Relative path overrides are resolved from the repository
root, including when the benchmark process runs from another working directory.
With no `FORNAX_BENCH_RAW`, every golden row must be materialized and is
benchmarked. `FORNAX_BENCH_RAW` selects exactly one input, but that canonical
path must belong to the selected golden manifest.

The benchmark rejects missing files, LFS pointer metadata, incorrect file sizes,
malformed manifests, duplicate paths, and paths outside the manifest before
timing starts. The old arbitrary single-fixture default is removed.

Materialize and verify the golden set before a local run:

```bash
pixi run -e dev cargo run --release -q -p decoder-tracker -- hash \
  --manifest crates/decoder-tracker/golden/selected-manifest.csv \
  --check crates/decoder-tracker/golden/decode-hashes.csv
```

`FORNAX_BENCH_DECODER` accepts `fornax`, `rawler`, and `flashraw`; the default
is all three. An explicit `FORNAX_BENCH_DECODER` value also controls CI; only a
CI run with no explicit selection defaults to Fornax alone. Every Fornax crop
case uses `f32`.

```bash
RAYON_NUM_THREADS=1 cargo bench -p fornax-decoder
```

FlashRaw is configured for the CPU, one worker thread, strict corruption
handling, no default crop, and no DNG opcode application. CI also sets
`RAYON_NUM_THREADS=1` and `OMP_NUM_THREADS=1`.

Compile the benchmark without executing timings:

```bash
cargo bench -p fornax-decoder --no-run
```

## Decoder output equality

Before trusting DNG timing ratios, compare the native U8/U16/U32/F32 raw mosaic
from Fornax, Rawler, and FlashRaw. The equality check uses exact dimensions,
storage type, and sample bits. It fails rather than skipping when a selected
reference cannot expose a DNG raw mosaic.

With the full golden set materialized:

```bash
RAYON_NUM_THREADS=1 OMP_NUM_THREADS=1 \
cargo test -p fornax-raw-decoder --test decoder_equivalence -- \
  --ignored --nocapture
```

Select one golden input with `FORNAX_BENCH_RAW` in the same way as the
benchmark. The benchmark runs the same three-way equality check for its
materialized DNG input before timing, so the CI matrix does not invoke the
standalone ignored test a second time.

## CI matrix

The `decoder-benchmark-plan` CI job reads the golden CSV and emits one matrix
item per file. Each `decoder-benchmark` job:

1. initializes the pointer-only raw.pixls.us submodule;
2. fetches only that matrix item's Git LFS object;
3. verifies its recorded size and SHA-256;
4. builds `fornax-decoder` with `--no-default-features` and only that input's
   vendor feature, except DNG which needs no format feature;
5. verifies exact native pixel equality across Fornax, Rawler, and FlashRaw;
6. runs Fornax, Rawler, and FlashRaw on that one input.

Changed-path impact selection is proportional. A direct vendor crate change
runs only that vendor's golden cases; `fornax-bmff` runs Canon cases; every
`fornax-codec` change runs all formats and DNG. Shared TIFF/DNG/core/decoder,
benchmark source, manifests, matrix scripts, and workflow changes run every
case.

RAW payloads remain transient. The decoder benchmark workflow does not cache or
upload them. Before registering a reference, the benchmark preflights both
decode and raw-mosaic support. Only typed `DecodeError::Unsupported` skips that
reference with a visible case/reference warning; backend errors, malformed
output, and panics fail the matrix case.

## Comparison scope

- Use `fornax/end_to_end` versus Rawler and FlashRaw `decode` as the headline
  end-to-end raw-decode ratio. Compare every file independently.
- Keep parse-only `fornax/decode`, pixel-only `fornax/raw_mosaic`,
  `cropped_mosaic`, and metadata diagnostics separate from the headline.
- Do not time reference `raw_mosaic` methods. Their decode calls already return
  decoded samples; the adapter methods only clone those samples into Fornax's
  benchmark types.
- Record the command and corpus revision in review evidence; do not commit
  host-dependent timing baselines.
- Inspect every single-file result. A favorable aggregate must not hide a
  regression in one format or compression family.
- Do not merge measurement stages into one Criterion item.
- Compare `dng_metadata` and `process_metadata` only with another implementation
  that exposes equivalent metadata. The current reference adapters do not.
- `rawler` and `flashraw` are dev-dependencies of `fornax-decoder`. They must not
  appear in the normal dependency graph of any production decoder crate.

See [FlashRaw reference notes](../reference/decoders/flashraw.md) for the pinned
benchmark adapter contract and known API differences.
