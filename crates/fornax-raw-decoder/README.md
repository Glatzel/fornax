# fornax-raw-decoder

The default native decoder. DNG handling is implemented by `fornax-dng`.
Supported Sony, Canon, Nikon, Fujifilm, Hasselblad, Panasonic, and Phase One
implementations are unconditional submodules and are exposed as
`fornax_raw_decoder::{sony, canon, nikon, fujifilm, hasselblad, panasonic,
phase_one}`. Generic BMFF/TIFF parsing and generic codecs remain in their
container-neutral crates.

There are no per-format features. `--no-default-features` still includes every
supported native format:

```bash
cargo build -p fornax-raw-decoder --no-default-features
```

The independent `mmap` and `zstd` features select infrastructure support; they
do not gate a vendor decoder.

```rust
use fornax_raw_core::{BayerPattern, MonoImage};

let mosaic = MonoImage::from_raw(2, 2, vec![0.0_f32; 4]).unwrap();
let pattern = BayerPattern::RGGB;
assert_eq!(pattern.channel_at(0, 0), fornax_raw_core::BayerChannel::R);
assert_eq!(mosaic.dimensions(), (2, 2));
```

## benchmark

Compares the default pure-Rust decoder with benchmark-only Rawler and FlashRaw
adapters over `decoder-tracker`'s manual performance set. The default manifest
contains stable perf representatives for every tier-0/1 lossless decoder path;
small rows explicitly marked `overhead_only` measure fixed-cost behavior.

Every input is a separate Criterion group. Fornax measures `end_to_end`,
parse-only `decode`, `raw_mosaic`, and `f32` `cropped_mosaic` items and
additionally reports `dng_metadata` and `process_metadata` diagnostics. Rawler
and FlashRaw measure only `decode`, whose decoded output is the reference
end-to-end result. Their raw-mosaic methods are checked during preflight but are
not timed because they only clone the already-decoded samples into Fornax's
benchmark types.

```bash
RAYON_NUM_THREADS=1 cargo bench -p fornax-raw-decoder
```

The benchmark reads
`crates/decoder-tracker/golden/perf-manifest.csv` and expects its files to
be materialized under `external/raw.pixls.us`. Override the manifest or corpus
root with `FORNAX_BENCH_MANIFEST` or `FORNAX_BENCH_CORPUS`. Select exactly one
golden member with:

```bash
FORNAX_BENCH_RAW=external/raw.pixls.us/path/from/perf-manifest.csv \
  cargo bench -p fornax-raw-decoder
```

The default `FORNAX_BENCH_GROUP=core` selects the short performance set.
Use `full`, `overhead`, `hdr`, a comma-separated combination, or `all` to
select other members before applying `FORNAX_BENCH_RAW`.

`FORNAX_BENCH_DECODER` selects `fornax`, `rawler`, `flashraw`, or a
comma-separated list. An unset or empty value benchmarks all three:

```bash
FORNAX_BENCH_DECODER=fornax \
  cargo bench -p fornax-raw-decoder
```

Before timing, the benchmark verifies the Fornax raw hash against the shared
golden hash file. CI creates one benchmark job per perf file, restores the
shared test/perf LFS cache, verifies the object SHA-256, and builds the decoder
without default infrastructure features. Vendor formats are always available.
A reference that reports typed unsupported decode or raw-mosaic support is
skipped with a visible warning; backend errors, hash mismatches, and invalid
output fail the job.

Compile the benchmark without running timings:

```bash
cargo bench -p fornax-raw-decoder --no-run
```
