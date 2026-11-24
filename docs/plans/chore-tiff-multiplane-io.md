# Chore: Multi-Channel & Multi-Plane TIFF Image IO and Official Corpus Testing

## Goal

Extend `fornax-tiff` (behind its `raster` feature) to support general TIFF image read and write IO for all five `fornax-core` image types (`MonoImage`, `UvImage`, `RgbImage`, `RgbaImage`, `MultiPlaneImage`) across `u8`, `u16`, `u32`, `f32`, and `f64`, and validate against the official LibTIFF test image corpus (`external/libtiff-pics`).

## Scope

### In
- `fornax-codec`:
  - Strided horizontal integer predictor (`Predictor::HorizontalIntegerStrided { samples_per_pixel }`) for chunky multi-sample images per TIFF 6.0 Section 14.
  - Arbitrary `byte_stride >= 1` for `Predictor::FloatingPoint` (e.g., `byte_stride = 3` for chunky RGB).
  - Fix partial final strip height in `deflate`, `lzw`, `packbits`, `zstd` decode and `encode_image` so partial final strips match TIFF 6.0 without padding errors.
- `fornax-tiff`:
  - Optional `fornax-core` and `image` dependencies gated behind `raster`.
  - `TiffTag::ExtraSamples`, `PlanarConfiguration`, `SampleFormatTag` constants, and `ParsedTiff::byte_order()`.
  - Planar-aware `RasterLayout` and TIFF 6.0 default `RowsPerStrip`.
  - Unified image IO API across `image.rs`, `reader.rs`, and `writer.rs` (`TypedTiffImage<T>`, `TiffImage::{open, from_bytes, save, write_to}`, `TiffIoError`, `ParsedTiff::{decode_image, decode_planes}`, `TiffWriter::write_image`, `TiffWriteOptions`, `WriteStorage`).
  - Comprehensive synthetic round-trip matrix tests (`tests/image_io.rs`) and `external/libtiff-pics` corpus tests (`tests/libtiff_pics.rs`).
- External submodule: `external/libtiff-pics` (`https://gitlab.com/libtiff/libtiff-pics.git`, shallow).

### Out
- Legacy / non-linear TIFF color spaces (`Palette`, `YCbCr`, `CieLab`, `LogLuv`) and legacy compressions (`Fax3`/`Fax4`, `OldJPEG`) — these return typed `TiffIoError::Unsupported` / `TiffIoError::Compression`.
- Changing `fornax-raw` DNG/vendor decoder behavior or `SampleBuffer`'s single-band contract.

## Task checklist

- [x] Create worktree `chore/tiff-multiplane-io`, link `external/raw-images`, init `external/raw.pixls.us`, and add `external/libtiff-pics` submodule.
- [x] Implement strided horizontal integer predictor, arbitrary float byte stride, and partial final strip height fixes in `fornax-codec`.
- [x] Implement `TypedTiffImage<T>`, `TiffImage`, `read_*`, and `write_*` in `fornax-tiff::io` with planar-aware `RasterLayout`.
- [x] Add synthetic round-trip tests (`tests/image_io.rs`) and `libtiff-pics` integration tests (`tests/libtiff_pics.rs`).
- [x] Run `cargo check -p fornax-tiff --no-default-features`, crate tests, and `scripts/agent-validation.sh`.

## Files expected to change

- `.gitmodules`, `external/libtiff-pics`, `scripts/setup.sh`, `scripts/agent-validation.sh`
- `crates/fornax-codec/src/predictor.rs`, `deflate.rs`, `lzw.rs`, `packbits.rs`, `zstd.rs`, `encode.rs`
- `crates/fornax-tiff/Cargo.toml`, `README.md`, `src/constants.rs`, `src/lib.rs`, `src/parser.rs`, `src/raster.rs`, `src/io.rs`
- `crates/fornax-tiff/tests/image_io.rs`, `crates/fornax-tiff/tests/libtiff_pics.rs`
- `docs/plans/chore-tiff-multiplane-io.md`

## Validation

- `cargo check -p fornax-tiff --no-default-features`
- `cargo test -p fornax-codec --all-features`
- `cargo test -p fornax-tiff --all-features`
- `scripts/agent-validation.sh`

## Decisions and notes

- `external/libtiff-pics` is the official LibTIFF test pictures repository; `image` with `features = ["tiff"]` is used as a dev-dependency oracle for cross-checking decoded pixels on supported files.
- `SampleBuffer` remains single-band (`MonoImage<T>`); chunky multi-channel images are decoded as `(width * samples_per_pixel) x height` and converted to `UvImage`, `RgbImage`, `RgbaImage`, or de-interleaved into `MultiPlaneImage`.
