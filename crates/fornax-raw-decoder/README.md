# fornax-decoder

Thin default decoder facade. DNG handling is implemented by `fornax-dng` and is
always enabled. The default feature set adds supported native handling from
`fornax-raw-sony`, `fornax-raw-canon`, `fornax-raw-nikon`,
`fornax-raw-fujifilm`, `fornax-raw-hasselblad`, `fornax-raw-panasonic`, and
`fornax-raw-phase-one`. Generic BMFF/TIFF parsing and generic codecs remain in
their container-neutral crates; vendor stream implementations live with their
vendor decoders.

Format features are `sony`, `canon`, `nikon`, `fujifilm`, `hasselblad`,
`panasonic`, and `phase-one`. Disable defaults and enable only the formats a
consumer needs:

```bash
cargo build -p fornax-raw-decoder --no-default-features
cargo build -p fornax-raw-decoder --no-default-features --features canon
```

A known format whose feature is disabled returns `DecodeError::Unsupported`
naming that feature. It does not fall through to Adobe DNG Converter. The
`fornax-raw` facade remains full-featured and does not duplicate these features.

```rust
use fornax_raw_core::{BayerPattern, MonoImage};

let mosaic = MonoImage::from_raw(2, 2, vec![0.0_f32; 4]).unwrap();
let pattern = BayerPattern::RGGB;
assert_eq!(pattern.channel_at(0, 0), fornax_raw_core::BayerChannel::R);
assert_eq!(mosaic.dimensions(), (2, 2));
```

## benchmark

Compares the default pure-Rust decoder with benchmark-only Rawler and FlashRaw
adapters over `decoder-tracker`'s golden corpus. The default set contains 30
passing conventional 2x2 RGB Bayer files across eight native formats and both
accepted Hasselblad 3FR compression families.

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
`crates/decoder-tracker/golden/selected-manifest.csv` and expects its files to
be materialized under `external/raw.pixls.us`. Override the manifest or corpus
root with `FORNAX_BENCH_MANIFEST` or `FORNAX_BENCH_CORPUS`. Select exactly one
golden member with:

```bash
FORNAX_BENCH_RAW=external/raw.pixls.us/path/from/selected-manifest.dng \
  cargo bench -p fornax-raw-decoder
```

`FORNAX_BENCH_DECODER` selects `fornax`, `rawler`, `flashraw`, or a
comma-separated list. An unset or empty value benchmarks all three:

```bash
FORNAX_BENCH_DECODER=fornax \
  cargo bench -p fornax-raw-decoder
```

CI creates one benchmark job per golden file, fetches only that file's
transient LFS object, verifies its SHA-256, and builds only the decoder feature
required by that file. A reference that reports typed unsupported decode or
raw-mosaic support is skipped with a visible warning; backend errors and
invalid output fail the job.

Compile the benchmark without running timings:

```bash
cargo bench -p fornax-raw-decoder --no-run
```
