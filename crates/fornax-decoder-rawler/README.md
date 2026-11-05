# fornax-decoder-rawler

Decodes conventional 2x2 RGB Bayer RAW files through Rawler and exposes the
decoded mosaic and normalized metadata through Fornax's decoder trait.

```rust
use std::path::Path;

use fornax_core::protocol::IDecoder;
use fornax_decoder_rawler::RawlerDecoder;

let (image, metadata) =
    IDecoder::<Path, f32>::decode_active(&RawlerDecoder, Path::new("input.cr2"))?;
```
