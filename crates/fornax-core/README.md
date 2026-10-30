# fornax-core

Shared RAW pipeline types, errors, metadata, Bayer layouts, and decoder,
preprocessor, demosaic, and post-processor traits for Fornax.

```rust
use fornax_core::{BayerPattern, MonoImage};

let mosaic = MonoImage::from_raw(2, 2, vec![0.0_f32; 4]).unwrap();
let pattern = BayerPattern::RGGB;
assert_eq!(pattern.channel_at(0, 0), fornax_core::BayerChannel::R);
assert_eq!(mosaic.dimensions(), (2, 2));
```
