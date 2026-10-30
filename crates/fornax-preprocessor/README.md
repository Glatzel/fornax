# fornax-preprocessor

Normalizes an active Bayer mosaic using black and white levels while preserving
the single-channel RAW layout.

```rust
use fornax_core::protocol::IPreprocessor;
use fornax_preprocessor::Preprocessor;

let raw_image = Preprocessor.process(decoded_active, &metadata.levels)?;
```
