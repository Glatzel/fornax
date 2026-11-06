# fornax-devtool

Deterministic fixtures, paths, image export helpers, and benchmark
configuration shared by Fornax tests and development tools.

```rust
use fornax_devtool::{BenchParams, synthetic_mono_image};

let params = BenchParams::from_env();
let image = synthetic_mono_image::<f32>((1_500, 1_000), 1.0);
assert_eq!(image.dimensions(), (1_500, 1_000));
assert!(!params.sizes.is_empty());
```

`FORNAX_BENCH_SIZE` sets deterministic synthetic image sizes.
`FORNAX_BENCH_BITS` selects `32`, `64`, or a comma-separated list; the default
is `32`.
