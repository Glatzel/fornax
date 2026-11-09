# fornax-preprocessor

Compatibility package for the preprocessing stage. The implementation and
benchmarks live in `fornax-raw-pipeline`.

```rust
use fornax_core::protocol::IPreprocessor;
use fornax_preprocessor::Preprocessor;

let normalized = Preprocessor.process(decoded_active, &metadata.levels)?;
```

New code should import `fornax_raw_pipeline::Preprocessor` directly.
