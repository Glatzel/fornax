# Step 4: codec sample buffer

## Status

Approved and implemented. The multi-band or wider-sample container shape
remains a reserved decision and is not part of this implementation.

## Approved shape as implemented

The type is single-band and lives in
`crates/fornax-codec/src/sample_buffer.rs`:

```rust
type Mono<T> = ImageBuffer<Luma<T>, Vec<T>>;

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum SampleBuffer {
    U8(Mono<u8>),
    U16(Mono<u16>),
    U32(Mono<u32>),
    F32(Mono<f32>),
}
```

Each variant wraps `image::ImageBuffer<Luma<T>, Vec<T>>`. `Luma` denotes one
scalar channel and does not imply that the samples are luminance values. There
is no band-count API and there are no interleaved or planar variants.

Constructors:

```rust
pub fn from_u8(width: u32, height: u32, data: Vec<u8>) -> Result<Self, SampleBufferError>;
pub fn from_u16(width: u32, height: u32, data: Vec<u16>) -> Result<Self, SampleBufferError>;
pub fn from_u32(width: u32, height: u32, data: Vec<u32>) -> Result<Self, SampleBufferError>;
pub fn from_f32(width: u32, height: u32, data: Vec<f32>) -> Result<Self, SampleBufferError>;
```

Accessors and consuming raw-vector accessors:

```rust
pub fn width(&self) -> u32;
pub fn height(&self) -> u32;
pub fn into_raw_u8(self) -> Option<Vec<u8>>;
pub fn into_raw_u16(self) -> Option<Vec<u16>>;
pub fn into_raw_u32(self) -> Option<Vec<u32>>;
pub fn into_raw_f32(self) -> Option<Vec<f32>>;
```

Errors:

```rust
#[derive(Clone, Debug, Eq, Error, PartialEq)]
#[non_exhaustive]
pub enum SampleBufferError {
    SampleCountOverflow { width: u32, height: u32 },
    SampleCountMismatch { width: u32, height: u32, expected: usize, actual: usize },
    InvalidDimensions { width: u32, height: u32 },
}
```

Construction converts both dimensions to `usize`, checks multiplication
overflow, requires the vector length to equal the image sample count, and then
calls `ImageBuffer::from_raw`.

`fornax-dng` privately owns the raw-workflow adapter in
`crates/fornax-dng/src/sample_buffer.rs`:

```rust
pub fn from_raw_mosaic(mosaic: &RawMosaic) -> SampleBuffer;
pub fn into_raw_mosaic(buffer: SampleBuffer) -> Result<RawMosaic, DecodeError>;
```

Codec decode returns an owned `SampleBuffer`. `fornax-dng` moves its variants
into `RawMosaic` without copying the sample vector at that boundary.
`EncodeRequest::image` borrows a `SampleBuffer`; the writer adapter therefore
clones `RawMosaic` data before encoding.

The public variants contain `image` 0.x types. An `image` 0.x version bump may
be semver-breaking for Fornax users who construct, match, or name those public
variant types, even when the nominal Patch version changes.
