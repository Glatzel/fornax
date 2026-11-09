# fornax-postprocessor

Compatibility package for demosaicing and RGB postprocessing. The
implementation and benchmarks live in `fornax-raw-pipeline`.

```rust
use fornax_core::protocol::IPostProcessor;
use fornax_postprocessor::{OutputColorSpace, PostProcessor};

let processor = PostProcessor {
    output_color: OutputColorSpace::Srgb,
    ..PostProcessor::default()
};
let rgb = processor.process(&normalized_image, &metadata)?;
```

New code should import these items from `fornax_raw_pipeline` directly.
