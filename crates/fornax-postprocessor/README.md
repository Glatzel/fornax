# fornax-postprocessor

Post-processes normalized Bayer data into RGB output, including Fornax's linear,
patterned-pixel-grouping, ratio-corrected, and adaptive homogeneity-directed
demosaic implementations.

```rust
use fornax_core::protocol::IPostProcessor;
use fornax_postprocessor::PostProcessor;

let processor = PostProcessor::default();
let rgb = processor.process(&raw_image, &metadata)?;
```
