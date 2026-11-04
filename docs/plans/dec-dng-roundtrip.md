# Implement DEC-DNG round-trip decoding and export

**Task:** `DEC-DNG`

## Goal

Implement the first working `fornax-decoder` DNG path for the accepted
`external/raw-images/images/colorchart-5D2-6000K.dng` fixture. Decoding must
produce the full 5,792 x 3,804 mosaic (GBRG at the full-frame origin; RGGB in
tag coordinates relative to `ActiveArea`) and preserve its root and raw-IFD
metadata. Writing with the default lossless-JPEG options must reproduce the
input file byte for byte.

## Scope

In scope:

- Classic TIFF DNG directory discovery through root IFDs and `SubIFDs`.
- Selection of the full-resolution CFA image IFD rather than the first preview.
- Lossless JPEG-92 tiled CFA decoding for the accepted fixture, including
  restart-free Huffman decoding and all seven JPEG-92 predictors.
- Typed metadata extraction from root and raw IFDs into the existing
  `DngMetadata` map while retaining tiff value types.
- CFA metadata validation and conversion to a conventional 2x2
  `fornax_core::BayerPattern`.
- Default lossless-JPEG export by preserving the original DNG container bytes
  and encoded tile payloads.
- Uncompressed DNG export from the decoded mosaic as a semantic fallback.
- Explicit errors for unsupported compression, storage, sample types, and
  malformed metadata.

Out of scope:

- BigTIFF and non-DNG containers.
- Deflate, JPEG XL, lossy JPEG, opcode lists, linearization tables, and
  interleave factors beyond the accepted fixture.
- General lossless-JPEG recompression when the source encoding is unavailable.
- Preview, thumbnail, and Exif image decoding.
- X-Trans, four-color, monochrome, and other non-2x2 Bayer handoff.
- Committing the external RAW fixture or any generated binary output.

## Task checklist

- [x] Add an integration test that decodes the accepted fixture, checks its
      dimensions, CFA, levels, and representative metadata, writes it with
      `DngWriteOptions::default()`, and compares every output byte with the
      input.
- [x] Run the test and confirm it fails at the current unimplemented decode
      path for the expected reason.
- [x] Implement `DecodedRaw::new` dimension validation and CFA validation at
      the handoff boundary.
- [x] Implement classic TIFF directory traversal and raw-IFD selection.
- [x] Implement typed root/raw metadata preservation.
- [x] Implement lossless JPEG-92 tile decoding.
- [x] Preserve the original container for default lossless-JPEG export.
- [x] Add uncompressed DNG export and verify it decodes back to identical
      samples and required tags.
- [x] Implement the 2x2 Bayer preprocessor handoff and reject other CFA sizes.
- [x] Keep `DEC-DNG` in `TODO.md` because the broader DNG 1.7.1.0 definition of
      done remains incomplete, update the crate documentation, and record
      validation results in this plan.

## Files expected to change

- `TODO.md`
- `Cargo.lock`
- `crates/fornax-decoder/Cargo.toml`
- `crates/fornax-decoder/README.md`
- `crates/fornax-decoder/src/decoded.rs`
- `crates/fornax-decoder/src/error.rs`
- `crates/fornax-decoder/src/lib.rs`
- `crates/fornax-decoder/src/metadata.rs`
- `crates/fornax-decoder/src/protocol.rs`
- `crates/fornax-decoder/src/writer.rs`
- `crates/fornax-decoder/src/dng/*.rs`
- `crates/fornax-decoder/tests/dng_roundtrip.rs`
- `docs/plans/dec-dng-roundtrip.md`

## Validation

Run from this worktree:

```bash
cargo test -p fornax-decoder --test dng_roundtrip
cargo test -p fornax-decoder
cargo test --workspace --all-features
cargo fmt --check
cargo clippy --workspace --all-features --all-targets -- \
    -A clippy::too_many_arguments \
    -D warnings
cargo tree -p fornax-decoder
git diff --check
```

The exact workspace clippy gate from the repository `AGENTS.md` is run before
handoff. Any pre-existing failure is reported explicitly rather than treated as
a pass.

Observed in this worktree:

- `cargo test -p fornax-decoder`: 25 passed across 3 suites.
- `cargo test --workspace --all-features`: 297 passed across 26 suites.
- `cargo +nightly fmt --check`: passed.
- `cargo clippy -p fornax-decoder --all-targets -- <repository lint flags>`:
  passed.
- The exact workspace clippy command still reports pre-existing findings in
  `fornax-decoder-rawler`, `fornax-preprocessor` benchmarks, and
  `fornax-postprocessor`; none is in code changed by this task.
- `cargo tree -p fornax-decoder`: pure Rust, with no C/C++ or `-sys` direct
  dependency.
- `git diff --check`: passed.

## Decisions and notes

- Byte-identical lossless-JPEG export requires retaining the source container.
  The decoded sample model alone cannot reconstruct tile boundaries, entropy
  coding choices, metadata ordering, or preview IFDs byte for byte.
- The source-preserving path is valid only while the decoded samples and
  metadata are unchanged. The current public API exposes immutable decoded
  samples, so default export can safely reuse the source bytes.
- Uncompressed output is regenerated from samples and regenerates offsets for
  its new layout. It is not expected to be byte-identical to a compressed
  source, but it must round-trip sample values and required CFA metadata.
- The accepted fixture has 345 256 x 256 lossless-JPEG tiles and a 2x2 RGGB
  pattern relative to its active area. It uses root preview and Exif IFDs, so
  first-IFD decoding is not acceptable.
- Lossless-JPEG support is limited initially to integer CFA tiles with one
  output sample per pixel and no restart markers. The entropy decoder accepts
  the one-to-four interleaved JPEG components used to represent CFA samples.
  Predictors 1 through 7 are implemented so the core JPEG-92 predictor variants
  share one checked decoder.
- The writer remains pure Rust and uses the existing `tiff` crate for the
  regenerated uncompressed container.
- The full `DEC-DNG` plan still requires deflate, JPEG XL, lossy JPEG,
  linearization/opcode handling, pre-1.1 lossless-JPEG compatibility, corpus
  coverage, and general lossless-JPEG recompression. The backlog item therefore
  remains open rather than being marked complete by this first vertical slice.
- Active-area decode shifts the full-frame Bayer pattern by the crop origin.
  For this fixture that changes the full-frame GBRG pattern back to RGGB after
  cropping at `(158, 51)`.
- Multi-strip and multi-tile uncompressed tests cover partial final strips,
  full-image scratch indexing, zero dimensions, and offset-overflow rejection.
