# fornax-decoder

Thin default decoder facade. DNG handling is implemented by `fornax-dng`;
audited lossless Canon CR3/CRM handling is implemented by `fornax-cr3`;
supported Sony ARW/NRW handling is implemented by `fornax-arw`; supported Nikon
NEF handling is implemented by `fornax-nef`; and supported Fujifilm RAF
handling is implemented by `fornax-raf`; supported Hasselblad 3FR handling is
implemented by `fornax-hasselblad`; supported Phase One/Leaf IIQ handling is
implemented by `fornax-iiq`; and supported Panasonic RW2 handling is implemented
by `fornax-rw2`. Generic BMFF/TIFF parsing, shared codecs, metadata conversion,
and writing remain in their container-neutral or format-specific crates.

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

Every input is a separate Criterion group. All decoders measure `decode`,
`raw_mosaic`, and `cropped_mosaic`. Fornax additionally reports
`dng_metadata` and `process_metadata` diagnostics; the reference adapters do not
expose equivalent metadata.

```bash
RAYON_NUM_THREADS=1 cargo bench -p fornax-decoder
```

The benchmark reads
`crates/decoder-tracker/golden/selected-manifest.csv` and expects its files to
be materialized under `external/raw.pixls.us`. Override the manifest or corpus
root with `FORNAX_BENCH_MANIFEST` or `FORNAX_BENCH_CORPUS`. Select exactly one
golden member with:

```bash
FORNAX_BENCH_RAW=external/raw.pixls.us/path/from/selected-manifest.dng \
  cargo bench -p fornax-decoder
```

`FORNAX_BENCH_BITS` selects `32`, `64`, or a comma-separated list. The default
is `32`.

`FORNAX_BENCH_DECODER` selects `fornax`, `rawler`, `flashraw`, or a
comma-separated list. An unset or empty value benchmarks all three:

```bash
FORNAX_BENCH_DECODER=fornax FORNAX_BENCH_BITS=32 \
  cargo bench -p fornax-decoder
```

CI creates one benchmark job per golden file, fetches only that file's
transient LFS object, verifies its SHA-256, and runs all three decoders on the
same runner.

Compile the benchmark without running timings:

```bash
cargo bench -p fornax-decoder --no-run
```
