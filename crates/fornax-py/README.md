# fornax-py

Python bindings for the Fornax RAW processing pipeline, built with PyO3 and
Maturin.

```bash
maturin develop --manifest-path crates/fornax-py/Cargo.toml
```

The binding currently targets Python 3.13 and is excluded from the Rust
workspace while its public API is stabilized.
