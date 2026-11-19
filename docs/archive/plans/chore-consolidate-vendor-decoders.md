# Consolidate vendor decoders into the raw decoder

## Goal

Move the Sony, Canon, Nikon, Fujifilm, Hasselblad, Panasonic, and Phase One
format implementations into unconditional modules of `fornax-raw-decoder`, so
one crate always exposes and dispatches every supported native format.

## Scope

In scope:

- move each `fornax-raw-<vendor>` implementation into
  `fornax-raw-decoder/src/decoders/<vendor>`;
- expose the moved modules as `fornax_raw_decoder::<vendor>`;
- make every vendor implementation and dependency unconditional;
- remove per-vendor Cargo features and disabled-feature dispatch branches;
- move vendor unit/integration tests and consolidated dependencies into
  `fornax-raw-decoder`;
- update decoder architecture, workflow, benchmark-impact selection, CI path
  filters, and validation package lists;
- update `fornax-raw-core` source-consistency checks to the new module paths.

Out of scope:

- demosaic or pipeline behavior;
- generic codec features such as Deflate, JPEG XL, mmap, and Zstandard;
- decoder output or metadata changes.

## Task checklist

- [x] Add a failing public-module test for every moved vendor decoder.
- [x] Move vendor source trees under `fornax-raw-decoder/src/decoders/`.
- [x] Rewrite module-local `crate::` paths and public doc examples.
- [x] Consolidate dependencies and remove per-vendor Cargo features.
- [x] Remove `cfg(feature = "...")` dispatch and disabled-feature behavior.
- [x] Move vendor integration tests and update facade imports.
- [x] Update benchmark impact selection and its tests.
- [x] Update CI path filters and validation commands.
- [x] Update current architecture and crate documentation.
- [x] Run focused tests, golden hashes, formatting, clippy, rustdoc, and
  `scripts/agent-validation.sh`.

## Files expected to change

- `Cargo.toml` and `Cargo.lock`
- `crates/fornax-raw-decoder/**`
- deleted `crates/fornax-raw-{canon,sony,nikon,fujifilm,hasselblad,panasonic,phase-one}/**`
- `crates/fornax-raw-core/tests/crop_cast.rs`
- `crates/decoder-tracker/scripts/benchmark_matrix.py`
- `crates/decoder-tracker/scripts/test_benchmark_impact.py`
- `crates/decoder-tracker/scripts/test_benchmark_matrix.py`
- `scripts/agent-validation.sh`
- `.github/workflows/ci.yml`
- `README.md`, `crates/fornax-raw-decoder/README.md`
- `docs/agent-workflow.md`, `docs/architecture/decoder.md`,
  `docs/architecture/overview.md`
- `docs/reference/decoders/operations/benchmark.md`

## Validation

```bash
cargo test -p fornax-raw-decoder
cargo test -p fornax-raw-decoder --no-default-features
cargo check -p fornax-raw-decoder --no-default-features
cargo bench -p fornax-raw-decoder --bench decoder --no-run
python3 crates/decoder-tracker/scripts/test_benchmark_matrix.py
python3 crates/decoder-tracker/scripts/test_benchmark_impact.py
python3 scripts/check-doc-links.py
scripts/agent-validation.sh
scripts/agent-validation.sh --docs-only
```

The tracked Nikon NEF and Canon CR3 golden rows must retain their existing
decoder hashes. `external/raw.pixls.us` remains untracked and untouched.

## Decisions and notes

- The public module names are `canon`, `sony`, `nikon`, `fujifilm`,
  `hasselblad`, `panasonic`, and `phase_one`; existing type names are retained.
- Vendor source remains separated internally by parser, codec, metadata, and
  tests. This is a crate/package consolidation, not a codec-layer merge.
- Format dispatch is unconditional. Known formats no longer have a
  disabled-feature error path.
- Generic `mmap` and `zstd` features remain because they select infrastructure
  support rather than a vendor decoder.
