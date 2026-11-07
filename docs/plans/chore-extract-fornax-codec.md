# Extract `fornax-codec` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> `superpowers:executing-plans` to implement this plan task-by-task.

**Goal:** Move format-neutral raw sample codecs into a reusable
`fornax-codec` crate with separate decode and encode traits, while keeping DNG
tag interpretation in `fornax-dng` and TIFF container mechanics in
`fornax-tiff`.

**Architecture:** `fornax-codec` owns raw sample decoding, chunk assembly,
predictors, and supported codec encoders. Public `ICodecDecode` and
`ICodecEncode` traits expose only supported directions. `fornax-dng` translates
TIFF/DNG metadata into format-neutral codec requests, then applies DNG
transforms, crop metadata, and container writing. `fornax-tiff` remains
container-only.

**Tech Stack:** Rust 2024, `fornax-core`, `rayon`, `flate2`, `zune-jpeg`,
`jxl-oxide`, optional `structured-zstd`, and `thiserror`.

## Scope

In scope:

- create `fornax-codec` and add it to the workspace;
- move Deflate, lossy JPEG, JPEG XL, lossless JPEG-92, Zstandard, and predictor
  implementations;
- move uncompressed packed, integer, float, and chunk-to-image sample decoding;
- expose `ICodecDecode` and `ICodecEncode`;
- route DNG decoding and supported writing through the codec crate;
- preserve the public `fornax-dng` API and feature names;
- update architecture and ownership documentation.

Out of scope:

- adding JPEG or JPEG XL encoders;
- changing the DNG writer's structural tag policy;
- changing decoder behaviour, crop semantics, or supported compression modes;
- modifying `TODO.md`.

## Public Interfaces

```rust
pub enum Compression {
    Uncompressed,
    LosslessJpeg,
    Deflate,
    LossyJpeg,
    JpegXl,
    Zstd,
}

pub enum SampleFormat {
    Integer,
    Float,
}

pub enum ByteOrder {
    Little,
    Big,
}

pub enum Predictor {
    None,
    HorizontalInteger,
    FloatingPoint { byte_stride: usize },
}

pub struct Chunk {
    pub offset: usize,
    pub byte_count: usize,
}

pub struct DecodeRequest<'a> {
    pub source: &'a [u8],
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
    pub chunk_width: usize,
    pub chunk_height: usize,
    pub byte_order: ByteOrder,
    pub predictor: Predictor,
}

pub struct EncodedChunks {
    pub chunks: Vec<Vec<u8>>,
}

pub trait ICodecDecode {
    fn decode(&self, request: &DecodeRequest<'_>) -> Result<RawMosaic, CodecError>;
}

pub trait ICodecEncode {
    fn encode(&self, request: &EncodeRequest<'_>) -> Result<EncodedChunks, CodecError>;
}
```

`CodecError` has `InvalidInput` and `Unsupported` variants. Convenience
functions dispatch by `Compression`. Chunk order is row-major.

## Task Checklist

- [x] Task 1: Create the crate, shared request types, error type, traits, and
      uncompressed sample decoder.
- [x] Task 2: Move compressed decoders and predictors behind `ICodecDecode`.
- [x] Task 3: Move supported encoders behind `ICodecEncode`.
- [ ] Task 4: Route `fornax-dng` decode and write paths through
      `fornax-codec`, then remove superseded modules and dependencies.
- [ ] Task 5: Update crate documentation, architecture, and agent ownership
      notes, then run full validation.

### Task 1: Codec Contract and Uncompressed Decoding

**Files:**

- Create: `crates/fornax-codec/Cargo.toml`
- Create: `crates/fornax-codec/src/lib.rs`
- Create: `crates/fornax-codec/src/error.rs`
- Create: `crates/fornax-codec/src/request.rs`
- Create: `crates/fornax-codec/src/samples.rs`
- Create: `crates/fornax-codec/src/predictor.rs`
- Create: `crates/fornax-codec/src/uncompressed.rs`
- Create: `crates/fornax-codec/tests/decode_contract.rs`
- Modify: `Cargo.toml`

**Interfaces:**

- Produces the public request, error, trait, and compression types listed
  above.
- Produces `UncompressedCodec`, which implements both `ICodecDecode` and
  `ICodecEncode` at the high-level image boundary.

- [ ] **Step 1: Add the crate scaffold and failing contract tests.**

Tests must cover packed 10-bit rows, big-endian partial strips, float16
widening, output type selection, invalid chunk counts, and overflow-safe
truncation.

- [ ] **Step 2: Run the new tests and confirm they fail because the codec API
      and implementation do not exist.**

Run: `cargo test -p fornax-codec --test decode_contract`

- [ ] **Step 3: Implement the request model, traits, error type, and
      uncompressed decoder.**

Move packed-bit, float-widening, byte-copy, and chunk-to-image logic from
`fornax-dng`; do not copy DNG tag parsing.

- [ ] **Step 4: Run the contract tests and crate tests.**

Run: `cargo test -p fornax-codec --all-features`

- [ ] **Step 5: Commit the task.**

```bash
git add Cargo.toml Cargo.lock crates/fornax-codec docs/plans/chore-extract-fornax-codec.md
git commit -m "feat: add raw codec contract and uncompressed decoding"
```

### Task 2: Compressed Decoders and Predictors

**Files:**

- Create: `crates/fornax-codec/src/deflate.rs`
- Create: `crates/fornax-codec/src/jpeg.rs`
- Create: `crates/fornax-codec/src/jpegxl.rs`
- Create: `crates/fornax-codec/src/ljpeg.rs`
- Create: `crates/fornax-codec/src/zstd.rs`
- Modify: `crates/fornax-codec/src/lib.rs`
- Modify: `crates/fornax-codec/src/predictor.rs`
- Modify: `crates/fornax-codec/tests/decode_contract.rs`

**Interfaces:**

- Produces `LosslessJpegCodec`, `DeflateCodec`, `LossyJpegCodec`,
  `JpegXlCodec`, and feature-gated `ZstdCodec`.
- Produces `decode(Compression, &DecodeRequest<'_>)`.

- [ ] **Step 1: Add failing tests for Deflate predictors, lossless JPEG
      predictors and point transforms, lossy JPEG type checks, JPEG XL type
      checks, Zstandard feature gating, and malformed payloads.**

- [ ] **Step 2: Run the tests and confirm the missing codec implementations
      cause the expected compile or assertion failures.**

Run: `cargo test -p fornax-codec --all-features --test decode_contract`

- [ ] **Step 3: Move the existing decoder implementations behind
      `ICodecDecode` and route the compression dispatcher through concrete
      codec types.**

- [ ] **Step 4: Run codec tests with and without Zstandard.**

Run: `cargo test -p fornax-codec --all-features`

Run: `cargo test -p fornax-codec`

- [ ] **Step 5: Commit the task.**

```bash
git add crates/fornax-codec
git commit -m "feat: move compressed raw decoders into fornax-codec"
```

### Task 3: Supported Codec Encoders

**Files:**

- Create: `crates/fornax-codec/src/encode.rs`
- Modify: `crates/fornax-codec/src/deflate.rs`
- Modify: `crates/fornax-codec/src/ljpeg.rs`
- Modify: `crates/fornax-codec/src/uncompressed.rs`
- Modify: `crates/fornax-codec/src/zstd.rs`
- Modify: `crates/fornax-codec/src/lib.rs`
- Create: `crates/fornax-codec/tests/encode_contract.rs`

**Interfaces:**

- Produces `encode(Compression, &EncodeRequest<'_>) -> EncodedChunks`.
- Uncompressed, lossless JPEG, Deflate, and Zstandard implement
  `ICodecEncode`; JPEG and JPEG XL do not.

- [ ] **Step 1: Add failing round-trip tests for U8/U16 lossless JPEG,
      uncompressed integer and float samples, Deflate predictors, and
      feature-gated Zstandard chunks.**

- [ ] **Step 2: Run the tests and confirm encode support is missing.**

Run: `cargo test -p fornax-codec --all-features --test encode_contract`

- [ ] **Step 3: Implement chunk extraction, sample serialization, predictors,
      and compression for the supported encoders.**

- [ ] **Step 4: Run the encode contract and all codec tests.**

Run: `cargo test -p fornax-codec --all-features`

- [ ] **Step 5: Commit the task.**

```bash
git add crates/fornax-codec
git commit -m "feat: expose raw codec encoders"
```

### Task 4: DNG Integration and Cleanup

**Files:**

- Modify: `crates/fornax-dng/Cargo.toml`
- Modify: `crates/fornax-dng/src/lib.rs`
- Modify: `crates/fornax-dng/src/dng_decoder/mod.rs`
- Modify: `crates/fornax-dng/src/writer.rs`
- Delete: superseded `crates/fornax-dng/src/dng_decoder/*_codec.rs`
- Delete: `crates/fornax-dng/src/dng_decoder/ljpeg.rs`
- Delete: `crates/fornax-dng/src/ljpeg_writer.rs`
- Modify: `Cargo.toml`, `Cargo.lock`

**Interfaces:**

- Consumes all public codec types and dispatch functions from Tasks 1-3.
- Preserves `DngImage`, `raw_mosaic`, `cropped_mosaic`, `DngCompression`,
  `DngWriter`, and the `zstd` feature.

- [ ] **Step 1: Add a failing DNG test for TIFF-to-codec request mapping,
      including tile geometry, predictor conversion, byte order, sample format,
      and chunk order.**

- [ ] **Step 2: Run the targeted DNG test and confirm the mapping helper is
      missing.**

Run: `cargo test -p fornax-dng --all-features request`

- [ ] **Step 3: Implement the DNG request adapter, route decode and writer
      compression through `fornax-codec`, and delete superseded modules and
      dependencies.**

- [ ] **Step 4: Run DNG tests with and without Zstandard.**

Run: `cargo test -p fornax-dng --all-features`

Run: `cargo test -p fornax-dng`

- [ ] **Step 5: Commit the task.**

```bash
git add Cargo.toml Cargo.lock crates/fornax-dng
git commit -m "refactor: route DNG sample coding through fornax-codec"
```

### Task 5: Documentation and Full Validation

**Files:**

- Create: `crates/fornax-codec/README.md`
- Modify: `AGENTS.md`
- Modify: `crates/fornax-core/README.md`
- Modify: `crates/fornax-decoder/README.md`
- Modify: `crates/fornax-dng/README.md`
- Modify: `crates/fornax-tiff/README.md`
- Modify: `docs/architecture.md`
- Modify: `docs/decoder/architecture.md`
- Modify: `docs/plans/chore-extract-fornax-codec.md`

**Interfaces:**

- Documents the crate boundary and public traits without changing runtime
  behaviour.

- [ ] **Step 1: Update ownership documentation and add public API examples.**
- [ ] **Step 2: Run documentation tests.**

Run: `cargo test -p fornax-codec --doc --all-features`

- [ ] **Step 3: Verify dependency boundaries and benchmark compilation.**

Run: `cargo tree -p fornax-codec`

Run: `cargo tree -p fornax-tiff`

Run: `cargo bench -p fornax-dng --features zstd --bench writer --no-run`

- [ ] **Step 4: Run repository validation.**

Run: `scripts/agent-validation.sh`

- [ ] **Step 5: Commit the task and mark the plan complete.**

```bash
git add AGENTS.md crates docs
git commit -m "docs: document fornax-codec ownership"
```

## Files Expected to Change

- `Cargo.toml`, `Cargo.lock`, `AGENTS.md`
- `crates/fornax-codec/**`
- `crates/fornax-dng/Cargo.toml`
- `crates/fornax-dng/src/**`
- `crates/fornax-dng/README.md`
- `crates/fornax-core/README.md`
- `crates/fornax-decoder/README.md`
- `crates/fornax-tiff/README.md`
- `docs/architecture.md`
- `docs/decoder/architecture.md`

## Validation

```bash
cargo test -p fornax-codec --all-features
cargo test -p fornax-codec
cargo test -p fornax-codec --doc --all-features
cargo test -p fornax-dng --all-features
cargo test -p fornax-dng
cargo bench -p fornax-dng --features zstd --bench writer --no-run
cargo tree -p fornax-codec
cargo tree -p fornax-tiff
scripts/agent-validation.sh
```

## Decisions and Notes

- `fornax-codec` depends on `fornax-core` for `RawMosaic`; it does not depend
  on `fornax-tiff` or `fornax-dng`.
- `ICodecDecode` and `ICodecEncode` are separate so decode-only JPEG and JPEG XL
  do not expose meaningless encode operations.
- `Compression::Zstd` remains representable without the feature; dispatch
  returns `CodecError::Unsupported` when Zstandard is disabled.
- DNG owns compression identifier mapping, strip/tile tag extraction, unknown
  compression errors, transforms, linearization, crop semantics, and writer
  metadata policy.
- `DngCompression::LosslessJpeg` keeps its existing policy of JPEG-92 for
  U8/U16 and Deflate for U32/F32.
