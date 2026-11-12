# Codec Enum API Implementation Plan

**Goal:** Replace the separate codec direction traits and free dispatch
functions with one public `Codec` enum whose variants decode and encode
format-neutral sample chunks.

**Architecture:** `Codec` is the only public codec implementation surface.
Each compression module remains an internal implementation detail. The enum
dispatches to decode/encode functions and carries the configurable Zstandard
compression level. `fornax-tiff::CompressionMethod` remains the TIFF wire
identifier, while `fornax-dng` maps it to `fornax-codec::Codec`.

**Tech Stack:** Rust 2024, `fornax-raw-core`, `fornax-codec`, `fornax-tiff`,
`fornax-dng`, and the existing optional `structured-zstd` dependency.

## Scope

In scope:

- Add `pub enum Codec` with the six supported compression schemes.
- Represent Zstandard as `Codec::Zstd { level: i32 }`.
- Add inherent `Codec::decode` and `Codec::encode` methods.
- Remove `ICodecDecode`, `ICodecEncode`, free `decode`/`encode` functions, and
  public concrete codec structs.
- Make codec implementation modules private.
- Map `fornax_tiff::CompressionMethod` to `Codec` only in `fornax-dng`.
- Preserve DNG `with_zstd_level` behavior and decoder feature handling.
- Update codec tests and public crate documentation.
- Move the compression-support matrix into `fornax-codec` and link to both
  `fornax-tiff` and `fornax-dng`.

Out of scope:

- Changing TIFF compression identifiers or moving `CompressionMethod`.
- Adding encode support for lossy JPEG or JPEG XL.
- Changing request/result layouts, DNG tags, payload semantics, or chunking.
- Adding a dynamic third-party codec extension mechanism.
- Modifying `TODO.md`.

## Task checklist

- [x] Task 1: Introduce `Codec` and migrate `fornax-codec` internals and tests.
- [x] Task 2: Migrate `fornax-dng` decode and writer paths to `Codec`.
- [x] Task 3: Update documentation, run focused tests, run workspace validation,
  inspect the diff, commit, and push to PR #660.

## Files expected to change

- `crates/fornax-codec/src/lib.rs`
- `crates/fornax-codec/src/request.rs`
- `crates/fornax-codec/src/uncompressed.rs`
- `crates/fornax-codec/src/ljpeg.rs`
- `crates/fornax-codec/src/deflate.rs`
- `crates/fornax-codec/src/jpeg.rs`
- `crates/fornax-codec/src/jpegxl.rs`
- `crates/fornax-codec/src/zstd.rs`
- `crates/fornax-codec/tests/decode_contract.rs`
- `crates/fornax-codec/tests/encode_contract.rs`
- `crates/fornax-codec/README.md`
- `crates/fornax-dng/README.md`
- `crates/fornax-dng/src/dng_decoder/mod.rs`
- `crates/fornax-dng/src/writer.rs`
- `docs/architecture.md`
- `docs/plans/chore-codec-enum-api.md`

## Review focus

- A decode-only `Codec` variant must return `CodecError::Unsupported` from
  `encode`, matching the previous dispatcher.
- `Codec::Zstd { level }` must use the level during encoding, ignore it during
  decoding, and remain runtime-gated when the `zstd` feature is disabled.
- TIFF compression parsing must remain in `fornax-tiff`, and `fornax-codec`
  must not gain a dependency on TIFF or DNG container types.
- DNG writing must preserve the level selected by `DngWriter::with_zstd_level`.
- Existing errors for malformed payloads and invalid request geometry must not
  change.

## Task 1: Public Codec Enum

**Files:**

- Modify: `crates/fornax-codec/src/lib.rs`
- Modify: `crates/fornax-codec/src/request.rs`
- Modify: `crates/fornax-codec/src/{uncompressed,ljpeg,deflate,jpeg,jpegxl,zstd}.rs`
- Modify: `crates/fornax-codec/tests/{decode_contract,encode_contract}.rs`
- Modify: `crates/fornax-codec/README.md`

**Interfaces:**

```rust
pub enum Codec {
    Uncompressed,
    LosslessJpeg,
    LossyJpeg,
    Deflate,
    JpegXl,
    Zstd { level: i32 },
}

impl Codec {
    pub fn decode(&self, request: &DecodeRequest<'_>) -> Result<RawMosaic, CodecError>;
    pub fn encode(&self, request: &EncodeRequest<'_>) -> Result<EncodedChunks, CodecError>;
}
```

- [ ] **Step 1: Rewrite contract tests against `Codec` methods.**

Use `Codec::Uncompressed.decode(&request)`, `Codec::Deflate.encode(&request)`,
`Codec::Zstd { level: 3 }.decode(&request)`, and equivalent calls for every
variant. Keep all existing malformed-input assertions and retain a test that
`Codec::LossyJpeg.encode` and `Codec::JpegXl.encode` return `Unsupported`.

- [ ] **Step 2: Run the codec tests and watch compilation fail because `Codec`
  methods do not exist.**

```bash
cargo -q test -p fornax-codec
```

Expected: unresolved imports or missing `Codec` methods.

- [ ] **Step 3: Implement the enum dispatch and private module functions.**

Rename `Compression` to `Codec`, add `Zstd { level: i32 }`, remove both public
traits and free dispatch functions, and expose only enum methods. Convert each
implementation module to provide crate-private `decode` and/or `encode`
functions; remove public marker structs where they have no remaining use.

- [ ] **Step 4: Run the codec tests with and without all features.**

```bash
cargo -q test -p fornax-codec
cargo -q test -p fornax-codec --all-features
```

Expected: all codec contract, unit, and documentation tests pass.

- [ ] **Step 5: Commit Task 1.**

```bash
git add crates/fornax-codec
git commit -m "refactor: expose codecs through one enum"
```

## Task 2: DNG Integration

**Files:**

- Modify: `crates/fornax-dng/src/dng_decoder/mod.rs`
- Modify: `crates/fornax-dng/src/writer.rs`

**Interfaces:**

- Consumes: `Codec`, `Codec::decode`, and `Codec::encode` from Task 1.
- Produces: `CodecPlan` and writer dispatch keyed by `Codec`.

- [ ] **Step 1: Change DNG tests and call sites to `Codec`.**

Update `CodecPlan`, compression-method mapping, decode calls, writer chunk
calls, and Zstandard writer calls. Map `DngCompression::Zstd` with the selected
`DngWriter::zstd_level` to `Codec::Zstd { level }`. Use level `3` for
DNG decoding because decoding does not consume the level.

- [ ] **Step 2: Run DNG tests and watch them fail against the removed names.**

```bash
cargo -q test -p fornax-dng
```

Expected: unresolved `Compression`, free function, or trait imports.

- [ ] **Step 3: Complete the DNG migration without changing its public API.**

- [ ] **Step 4: Run DNG tests with and without all features.**

```bash
cargo -q test -p fornax-dng
cargo -q test -p fornax-dng --all-features
```

Expected: all DNG unit, integration, and documentation tests pass.

- [ ] **Step 5: Commit Task 2.**

```bash
git add crates/fornax-dng
git commit -m "refactor: route DNG codecs through Codec"
```

## Task 3: Documentation and Validation

**Files:**

- Modify: `docs/architecture.md`
- Modify: `docs/plans/chore-codec-enum-api.md`
- Modify: `crates/fornax-codec/README.md`
- Modify: `crates/fornax-dng/README.md`

- [x] **Step 1: Update architecture documentation to name `Codec` as the public
  codec API and explain the TIFF `CompressionMethod` mapping. Move the
  compression-support matrix to `fornax-codec` and link it from
  `fornax-dng`.**

- [x] **Step 2: Run the repository validation script.**

```bash
scripts/agent-validation.sh
```

Expected: exit 0 with workspace tests, nightly formatting, documentation,
Clippy, decoder benchmark compilation, and diff checks passing.

- [x] **Step 3: Inspect the final diff for removed public traits, unchanged DNG
  behavior, and absence of a `fornax-codec -> fornax-tiff` dependency.**

- [x] **Step 4: Commit documentation and validation updates.**

```bash
git add docs/architecture.md docs/plans/chore-codec-enum-api.md
git commit -m "docs: document codec enum API"
```

- [x] **Step 5: Push the existing PR branch.**

```bash
git push origin HEAD
```

## Decisions and notes

- `CompressionMethod` remains in `fornax-tiff`; it models a TIFF tag value,
  while `Codec` models executable sample codecs.
- Zstandard carries its encode level because `DngWriter` already exposes that
  configuration publicly.
- A single `ICodec` trait was rejected because one method direction cannot be
  omitted for decode-only formats without runtime capability errors or dummy
  methods.
- No compatibility aliases are retained because `fornax-codec` has not shipped;
  only this PR and `fornax-dng` consume the current API.
- The codec implementation modules stay private even though their functions use
  `pub`; module privacy keeps them crate-internal while satisfying Clippy's
  redundant `pub(crate)` lint.
