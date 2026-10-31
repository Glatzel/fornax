# fornax-postprocessor

Post-processes normalized Bayer data into RGB output. The `Demosaic` selector
supports linear, PPG, RCD, AHD, VNG, DCB, DHT, AAHD, AMaZE, and LMMSE
reconstruction for conventional 2x2 Bayer images.

The VNG, DCB, DHT, AAHD, AMaZE, and LMMSE variants use the exact-reference
backend in `fornax-demosaic-reference`. That private crate links LibRaw and
compiles source adapted from pinned darktable files downloaded by the setup
scripts. These variants therefore carry LibRaw and darktable licensing terms;
the other Fornax implementations remain Rust-native.

```rust
use fornax_core::protocol::IPostProcessor;
use fornax_postprocessor::PostProcessor;

let processor = PostProcessor::default();
let rgb = processor.process(&raw_image, &metadata)?;
```
