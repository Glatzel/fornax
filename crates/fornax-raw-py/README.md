# fornax-py

Python bindings for the Fornax RAW processing pipeline, built with PyO3 and
Maturin.

```bash
maturin develop --manifest-path crates/fornax-py/Cargo.toml
```

The binding currently targets Python 3.13 and is excluded from the Rust
workspace while its public API is stabilized.

It currently cannot build. `crates/fornax-py/Cargo.toml` inherits the removed
workspace dependency `fornax` instead of `fornax-raw`, and the source imports
`fornax::dnc`, `fornax::fornax_dalim`, `fornax::libraw`, and
`fornax::Fornax`. Those modules and types are not exported by the current
facade. The intended Python API remains a
[reserved decision](../../docs/_archive/workspace-refactor/steps-1-6-review.md#11-decisions-and-gaps).
