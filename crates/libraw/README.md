# libraw

Safe Rust types for LibRaw's RAW metadata and processed-image APIs, plus a
`LibrawDecoder` adapter for Fornax's common decoder interface.

```rust
use std::path::Path;

use fornax_core::protocol::IDecoder;
use libraw::LibrawDecoder;

let (image, metadata) =
    IDecoder::<Path, f32>::decode_full(&LibrawDecoder, Path::new("input.cr2"))?;
```

Set `LIBRAW_ROOT` to a LibRaw installation containing `include/` and `lib/`.
