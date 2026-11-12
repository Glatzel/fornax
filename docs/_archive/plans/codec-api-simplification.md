# Simplify Codec and Dependency APIs

This archived plan documents the completed Simplify Codec and Dependency APIs work.

## Goal

Replace `fornax_codec::Codec` with direction-specific compression enums stored
in decode and encode requests, expose free `decode` and `encode` functions,
and make `fornax-cli` consume Fornax workspace APIs through the `fornax-raw`
facade. Audit workspace dependencies and gate the DNC decoder to its supported
platforms.

## Scope

In scope:

- Replace the combined codec selector with `DecodeCompression` and
  `EncodeCompression`.
- Add `compression` fields to `DecodeRequest` and `EncodeRequest`.
- Expose free `fornax_codec::decode` and `fornax_codec::encode` functions.
- Remove `Codec` and the runtime encode rejection for decode-only formats.
- Update DNG decode and writer integration.
- Move `fornax-cli` workspace APIs to the `fornax-raw` facade.
- Re-export the facade types needed by the CLI and its tests.
- Target-gate `fornax-decoder-dnc` and its implementation dependencies.
- Update current architecture and crate documentation.

Out of scope:

- Changing codec payload semantics, supported algorithms, chunk geometry, or
  compression identifiers.
- Replacing intentional direct dependencies in internal stage crates,
  benchmarks, or `compare-demosaic`.
- Changing `DngCompression`, `DngWriter`, CLI behavior, or feature semantics.
- Modifying `TODO.md` or the excluded `fornax-py` crate.

## Public Interfaces

```rust
pub enum DecodeCompression {
    Uncompressed,
    LosslessJpeg,
    Deflate,
    LossyJpeg,
    JpegXl,
    Zstd,
}

pub enum EncodeCompression {
    Uncompressed,
    LosslessJpeg,
    Deflate,
    Zstd { level: i32 },
}

pub struct DecodeRequest<'a> {
    pub source: &'a [u8],
    pub compression: DecodeCompression,
    pub width: usize,
    pub height: usize,
    pub chunk_width: usize,
    pub chunk_height: usize,
    pub chunks: &'a [Chunk],
    pub bits_per_sample: u16,
    pub sample_format: SampleFormat,
    pub byte_order: ByteOrder,
    pub predictor: Predictor,
}

pub struct EncodeRequest<'a> {
    pub image: &'a RawMosaic,
    pub compression: EncodeCompression,
    pub chunk_width: usize,
    pub chunk_height: usize,
    pub byte_order: ByteOrder,
    pub predictor: Predictor,
}

pub fn decode(request: &DecodeRequest<'_>) -> Result<RawMosaic, CodecError>;
pub fn encode(request: &EncodeRequest<'_>) -> Result<EncodedChunks, CodecError>;
```

`CodecError` remains unchanged. `DngWriter::with_zstd_level` remains the
public writer configuration mechanism.

## Task Checklist

- [x] Task 1: Add the implementation plan and worktree ledger.
- [x] Task 2: Migrate the `fornax-codec` API and contract tests.
- [x] Task 3: Migrate DNG decoding and writing to request-owned compression.
- [x] Task 4: Simplify the facade, CLI, and workspace dependency graph.
- [x] Task 5: Update documentation and run final validation.

### Task 1: Plan and Worktree Record

- Add this plan to the branch.
- Record the approved API and dependency rulings in the task ledger.
- Commit the plan without changing `TODO.md`.

### Task 2: Codec API

- Update contract tests first to construct requests with compression and call
  free `decode`/`encode` functions.
- Watch the tests fail against the old `Codec` API.
- Replace `Codec` with `DecodeCompression` and `EncodeCompression` in
  `request.rs`.
- Move dispatch into free functions in `lib.rs`.
- Preserve Zstandard feature-gating behavior and all existing round trips.
- Remove the now-impossible decode-only encoder runtime test.

### Task 3: DNG Integration

- Map TIFF compression identifiers to `DecodeCompression`.
- Remove the placeholder Zstandard decode level.
- Build encode requests with `EncodeCompression`, including the configured
  Zstandard level.
- Verify DNG tests with and without the `zstd` feature.

### Task 4: Facade, CLI, and Dependencies

- Re-export `OutputColorSpace` through `fornax_raw::postprocessor`.
- Re-export `TiffTag` through `fornax_raw::dng::metadata`.
- Replace the CLI's six direct Fornax dependencies with the facade enabled for
  `metadata-json` and `zstd`.
- Update all CLI source and test imports to facade paths.
- Keep direct external CLI dependencies used by the CLI itself.
- Target-gate the facade's DNC dependency and all DNC implementation
  dependencies to Windows and macOS.
- Leave other production, benchmark, and comparison dependencies unchanged
  after the audit.

### Task 5: Documentation and Validation

- Update `fornax-codec` and architecture documentation for request-owned
  compression.
- Verify the CLI dependency tree and platform-gated DNC dependency tree.
- Run focused and full repository validation.
- Inspect the final diff, commit all changes, push the branch, and open one
  semantic pull request.

## Files Expected to Change

- `Cargo.toml`
- `Cargo.lock`
- `crates/fornax-codec/src/lib.rs`
- `crates/fornax-codec/src/request.rs`
- `crates/fornax-codec/tests/{decode_contract,encode_contract}.rs`
- `crates/fornax-codec/README.md`
- `crates/fornax-dng/src/dng_decoder/mod.rs`
- `crates/fornax-dng/src/writer.rs`
- `crates/fornax-dng/src/metadata.rs`
- `crates/fornax-raw/Cargo.toml`
- `crates/fornax-raw/src/postprocessor.rs`
- `crates/fornax-decoder-dnc/Cargo.toml`
- `crates/fornax-cli/Cargo.toml`
- `crates/fornax-cli/README.md`
- `crates/fornax-cli/src/{config,convert,discover}.rs`
- `crates/fornax-cli/tests/cli.rs`
- `crates/fornax-raw/src/lib.rs`
- `docs/architecture/overview.md`
- `docs/_archive/plans/codec-api-simplification.md`

## Validation

```bash
cargo test -p fornax-codec
cargo test -p fornax-codec --all-features
cargo test -p fornax-dng
cargo test -p fornax-dng --all-features
cargo test -p fornax-raw --all-features
cargo test -p fornax-cli
cargo tree -p fornax-cli --depth 1 --edges normal
cargo tree -p fornax-raw --target "$(cargo -Vv | sed -n 's/^host: //p')" --edges normal
scripts/agent-validation.sh
```

Acceptance criteria:

- `fornax-codec` exposes no `Codec` type or methods.
- Decode-only formats cannot be selected for encoding.
- Zstandard decoding has no level and encoding carries its configured level.
- DNG decode/write behavior and error messages remain compatible.
- `fornax` is the only direct internal workspace dependency of `fornax-cli`.
- The host dependency tree does not contain `fornax-decoder-dnc` on Linux.
- CLI integration tests and workspace validation pass.

## Validation Results

- `scripts/agent-validation.sh`: passed all workspace tests and doctests,
  formatting, documentation, Clippy, benchmark compilation, and diff checks.
- Default features for `fornax-codec`, `fornax-dng`, `fornax-cli`, and
  `fornax`: 74 tests passed across 13 suites.
- All features for `fornax-codec` and `fornax-dng`: passed after focused runs.
- `cargo tree -p fornax-cli --depth 1 --edges normal`: `fornax` is the only
  direct internal workspace dependency.
- Host-target trees for `fornax` and `fornax-decoder-dnc` contain no DNC edge
  on Linux.

## Decisions and Notes

- The approved request API places compression in the request instead of adding
  a selector parameter to the free functions.
- Historical codec plans are records of merged work and are not rewritten.
- Internal stage crates keep direct dependencies to preserve architectural
  boundaries. The facade is for consumers, not a replacement for library
  composition.
- `fornax-cli` may directly depend on external crates it invokes; "use only
  fornax" applies to Fornax workspace crates.
- `fornax-py` remains excluded from the workspace.
- No compatibility alias for `Codec` is retained.
