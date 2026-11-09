# Add neutral codec sample buffer

## Goal

Move neutral codec output into a codec-owned `SampleBuffer`, remove
`fornax-codec`'s dependency on `fornax-core`, and preserve `RawMosaic` as the
storage-native raw workflow type through adapters at the DNG codec boundary.

## Scope

In scope:

- Add the maintainer-approved mono `SampleBuffer` to `fornax-codec`.
- Change generic codec decode and encode APIs to use `SampleBuffer`.
- Add and test conversion between `SampleBuffer` and `RawMosaic`.
- Update codec documentation, examples, contract tests, and dependency lists.

Out of scope:

- Band counts, planar/interleaved layouts, borrowed sample wrappers, or future
  sample types.
- Moving TIFF raster handling, DNG constants, or vendor codecs.
- Changing `RawMosaic` ownership, decoder APIs, workspace layout, or Step 5.
- Changing the codec's accepted storage types or encoded output.

## Approved shape

The storage container is `image::ImageBuffer<Luma<T>, Vec<T>>`:

```rust
use image::{ImageBuffer, Luma};

type Mono<T> = ImageBuffer<Luma<T>, Vec<T>>;

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum SampleBuffer {
    U8(Mono<u8>),
    U16(Mono<u16>),
    U32(Mono<u32>),
    F32(Mono<f32>),
}

impl SampleBuffer {
    pub fn from_u8(width: u32, height: u32, data: Vec<u8>)
        -> Result<Self, SampleBufferError>;

    pub fn from_u16(width: u32, height: u32, data: Vec<u16>)
        -> Result<Self, SampleBufferError>;

    pub fn from_u32(width: u32, height: u32, data: Vec<u32>)
        -> Result<Self, SampleBufferError>;

    pub fn from_f32(width: u32, height: u32, data: Vec<f32>)
        -> Result<Self, SampleBufferError>;

    pub fn width(&self) -> u32;
    pub fn height(&self) -> u32;

    pub fn into_raw_u8(self) -> Option<Vec<u8>>;
    pub fn into_raw_u16(self) -> Option<Vec<u16>>;
    pub fn into_raw_u32(self) -> Option<Vec<u32>>;
    pub fn into_raw_f32(self) -> Option<Vec<f32>>;
}
```

The buffer is single-band. `Luma` denotes one scalar channel and does not imply
that the samples are luminance values. Constructors use checked sample-count
validation and `ImageBuffer::from_raw`; raw extraction consumes the buffer
without copying. `SampleBuffer` is the only codec-owned storage enum, and no
band or interleaving abstraction is introduced.

## Adapter boundary

`RawMosaic` remains owned by `fornax-core`, and `fornax-core` does not depend on
`fornax-codec`. The compatibility adapters live in `fornax-dng`, which already
depends on both crates:

- Decode: codec returns `SampleBuffer`, then `fornax-dng` converts it to
  `RawMosaic` before the existing raw transforms and decoder contract.
- Encode: `DngWriter` converts its stable `RawMosaic` input to `SampleBuffer`
  once at the writer boundary, then passes the neutral buffer to the codec.
- A conversion back to `RawMosaic` rejects future unknown sample variants as
  unsupported rather than guessing.

ARW continues to construct `RawMosaic::U16` through its vendor decoder and does
not use the generic codec.

## Task checklist

- [ ] **Task 1: Add failing `SampleBuffer` contract tests.**
  - [ ] Add tests for construction and dimensions for U8, U16, U32, and F32.
  - [ ] Add tests for short and long input buffers.
  - [ ] Add tests for consuming raw extraction for every sample type and for
        mismatched extraction returning `None`.
  - [ ] Run the focused tests and confirm they fail because `SampleBuffer` does
        not exist.

- [ ] **Task 2: Implement the approved `SampleBuffer`.**
  - [ ] Add `sample_buffer.rs` with the exact enum, constructors, accessors,
        consuming extraction, and a documented `SampleBufferError`.
  - [ ] Use checked sample-count calculation and `ImageBuffer::from_raw`.
  - [ ] Export the public API and add rustdoc examples.
  - [ ] Run the focused tests to GREEN.

- [ ] **Task 3: Move codec decode and encode APIs to `SampleBuffer`.**
  - [ ] First update codec contract tests to expect `SampleBuffer`.
  - [ ] Watch the tests fail against the old `RawMosaic` API.
  - [ ] Change codec requests, result constructors, encoder input, internals,
        examples, and documentation to use `SampleBuffer`.
  - [ ] Remove `fornax-core` from `fornax-codec` dependencies.
  - [ ] Add a codec round-trip covering all four approved storage types.
  - [ ] Run `cargo test -p fornax-codec` to GREEN.

- [ ] **Task 4: Add `RawMosaic` compatibility adapters in DNG.**
  - [ ] Add tests for `RawMosaic` to `SampleBuffer` conversion for all four
        variants and exact F32 bit preservation.
  - [ ] Add tests for `SampleBuffer` to `RawMosaic` conversion for all four
        variants.
  - [ ] Watch the adapter tests fail before adding the adapter.
  - [ ] Convert codec decode output in DNG before existing raw transforms.
  - [ ] Convert `DngWriter` input once before codec encoding.
  - [ ] Keep the decoder and writer public signatures source-compatible.
  - [ ] Run `cargo test -p fornax-dng` to GREEN.

- [ ] **Task 5: Documentation and downstream verification.**
  - [ ] Update the codec README and crate-level documentation for the neutral
        buffer and DNG adapter boundary.
  - [ ] Update every affected codec example and test fixture.
  - [ ] Run all task-mandated tests and validation commands.
  - [ ] Run the golden decoder hash check and compare it with the committed
        baseline.
  - [ ] Inspect the complete diff, commit, push, and open one PR.

## Files expected to change

- `crates/fornax-codec/Cargo.toml`
- `crates/fornax-codec/README.md`
- `crates/fornax-codec/src/lib.rs`
- `crates/fornax-codec/src/request.rs`
- `crates/fornax-codec/src/sample_buffer.rs`
- `crates/fornax-codec/src/samples.rs`
- `crates/fornax-codec/src/encode.rs`
- `crates/fornax-codec/src/uncompressed.rs`
- `crates/fornax-codec/src/deflate.rs`
- `crates/fornax-codec/src/zstd.rs`
- `crates/fornax-codec/src/jpeg.rs`
- `crates/fornax-codec/src/jpegxl.rs`
- `crates/fornax-codec/src/ljpeg.rs`
- `crates/fornax-codec/tests/decode_contract.rs`
- `crates/fornax-codec/tests/encode_contract.rs`
- `crates/fornax-codec/tests/sample_buffer.rs`
- `crates/fornax-dng/src/dng_decoder/mod.rs`
- `crates/fornax-dng/src/writer.rs`
- `docs/plans/chore-codec-sample-buffer.md`

## Validation

```bash
cargo test -p fornax-codec
cargo test -p fornax-dng
cargo test -p decoder-tracker
cargo test --workspace --all-features --exclude compare-demosaic
scripts/agent-validation.sh
cargo run -q -p decoder-tracker -- hash \
  --manifest crates/decoder-tracker/golden/selected-manifest.csv \
  --check crates/decoder-tracker/golden/decode-hashes.csv
git diff --check
```

Also run focused tests after each RED-GREEN cycle and `cargo test --doc -p
fornax-codec` after adding public rustdoc examples.

## Decisions and notes

- The maintainer approved a single-band `ImageBuffer<Luma<T>, Vec<T>>` storage
  enum with U8, U16, U32, and F32 variants, checked construction through
  `from_raw`, and consuming zero-copy raw extraction.
- `SampleBuffer` remains `#[non_exhaustive]`; downstream matches must handle
  future variants explicitly.
- The DNG adapter boundary avoids both duplicating codec types in core and
  reintroducing a `fornax-codec -> fornax-core` dependency.
- No interleaved or planar invalid tests are added because the approved shape
  has no band or layout representation.
- `TODO.md` is not modified.
