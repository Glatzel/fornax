# fornax-raw-panasonic

Native Panasonic RW2 container and Bayer decoding.

`fornax-raw-panasonic` parses classic-TIFF RW2 files, selects the raw-data or
strip layout, dispatches Panasonic raw-format versions through its local codec,
and maps active area, default crop, CFA, black/white levels, as-shot neutral,
orientation, serial, lens, and DNG-compatible metadata.

V4/V5/V6/V8, stamped layouts, and validated V8 parameter tables live in this
crate behind a private `ICodecDecode` codec value.

Supported single-shot layouts are Panasonic raw-format V4, V5, V6, and V8 plus
stamped 12-bit storage. Decoded samples are returned as `RawMosaic::U16`.
Multi-shot markers, high-resolution composites, unknown versions, unknown
CFAs, and V7 return typed unsupported errors. V7 remains deferred until a real
single-shot V7 file can validate a codec.

```rust
use fornax_raw_panasonic::Rw2Image;

let bytes = std::fs::read("photo.RW2")?;
let decoder = Rw2Image::parse(&bytes)?;
assert!(decoder.layout_dimensions().0 > 0);
# Ok::<(), Box<dyn std::error::Error>>(())
```

Implementation details are recorded in the
[RW2 format plan](../../docs/decoder/formats/panasonic-rw2.md) and
[Panasonic reference note](../../docs/reference/decoders/panasonic-rw2.md).
