# fornax-decoder

Thin default decoder facade. DNG handling is implemented by `fornax-dng`;
supported Sony ARW/NRW handling is implemented by `fornax-arw`; and supported
Nikon NEF handling is implemented by `fornax-nef`; supported Fujifilm RAF
handling is implemented by `fornax-raf`; supported Phase One/Leaf IIQ handling
is implemented by `fornax-iiq`. Generic TIFF parsing, shared codecs, DNG
metadata conversion, and writing remain in their format-neutral or DNG-specific
crates.

```rust
use fornax_raw_core::{BayerPattern, MonoImage};

let mosaic = MonoImage::from_raw(2, 2, vec![0.0_f32; 4]).unwrap();
let pattern = BayerPattern::RGGB;
assert_eq!(pattern.channel_at(0, 0), fornax_raw_core::BayerChannel::R);
assert_eq!(mosaic.dimensions(), (2, 2));
```

## benchmark

Compares the default pure-Rust decoder with the Rawler adapter through the
shared `IDecoder` contract.

The benchmark measures the five `IDecoder` methods:

- `decode`;
- `raw_mosaic`;
- `cropped_mosaic`;
- `dng_metadata`;
- `process_metadata`.

```bash
cargo bench -p fornax-decoder
```

The default fixture is `external/raw-images/images/colorchart-5D2-6000K.dng`.
Select another file with:

```bash
FORNAX_BENCH_RAW=/path/to/file.dng cargo bench -p fornax-decoder
```

`FORNAX_BENCH_BITS` selects `32`, `64`, or a comma-separated list. The default
is `32`.

`FORNAX_BENCH_DECODER` selects `fornax`, `rawler`, or a comma-separated list.
An unset or empty value benchmarks both:

```bash
FORNAX_BENCH_DECODER=fornax FORNAX_BENCH_BITS=32 \
  cargo bench -p fornax-decoder
```

The benchmark prints each decoder's DNG metadata tag count before timing its
metadata accessor. Compare metadata timings only when both decoders expose
equivalent tag coverage.

Compile the benchmark without running timings:

```bash
cargo bench -p fornax-decoder --no-run
```
