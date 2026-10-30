# fornax-decoder-dnc

Decodes RAW files by converting them with Adobe DNG Converter and then reading
the resulting DNG with Rawler. DNC is available on Windows and macOS only.

```rust
use std::path::Path;

use fornax_core::protocol::IDecoder;
use fornax_decoder_dnc::{DncDecoder, DncPreview};

let decoder = DncDecoder {
    preview: DncPreview::Full,
    ..DncDecoder::default()
};
let (image, metadata) = IDecoder::<Path, f32>::decode_active(&decoder, Path::new("input.cr2"))?;
```
