# fornax-postprocessor

Post-processes normalized Bayer data into RGB output. The `Demosaic` selector
supports linear, PPG, RCD, AHD, VNG, DCB, DHT, AAHD, AMaZE, and LMMSE
reconstruction for conventional 2x2 Bayer images.

All ten demosaic variants are implemented in Rust. LibRaw and darktable are
used only by the separate comparison crate as behavioral references.

```rust
use fornax_core::protocol::IPostProcessor;
use fornax_postprocessor::PostProcessor;

let processor = PostProcessor::default();
let rgb = processor.process(&raw_image, &metadata)?;
```
