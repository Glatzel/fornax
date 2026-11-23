# fornax-raw

Fornax's high-level pure-Rust RAW crate. It owns shared image and metadata
contracts, DNG decoding and writing, native vendor decoders, preprocessing,
demosaicing, and post-processing. Vendor modules are exposed as
`fornax_raw::canon`, `fornax_raw::sony`, and the corresponding format modules.

The processing handoff accepts conventional 2x2 RGB Bayer data only. X-Trans
and other CFA layouts may remain available through decoder/DNG paths, but must
not enter preprocessing or demosaicing.

Decode and process one RAW:

```rust,no_run
use fornax_raw::protocol::IDecoder;
use fornax_raw::{Decoder, Preprocessor};

let decoder = Decoder::decode("photo.dng".as_ref())?;
let cropped = decoder.cropped_mosaic::<f32>()?;
let metadata = decoder.process_metadata()?;
let normalized = cropped.process(&Preprocessor, &metadata.levels)?;
# Ok::<(), fornax_raw::RawError>(())
```

DNG writing is available through the dedicated writer module:

```rust
use fornax_raw::writer::{DngWriter, DngWriterCompression};

let writer = DngWriter::new(DngWriterCompression::Lossless);
assert_eq!(writer.compression(), DngWriterCompression::Lossless);
```

The `fornax-raw-cli` package provides batch conversion:

```bash
cargo run -p fornax-raw-cli -- --format jpeg path/to/image.dng
```
