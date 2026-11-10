# DEC-ARW Sony ARW/NRW decoder

## Goal

Implement the `DEC-ARW` backlog item as a native, pure-Rust Sony ARW/NRW
decoder that returns the same mosaic and metadata interfaces as the default
decoder, integrates with the facade and CLI, and removes the completed item
from `TODO.md`.

## Scope

In scope:

- classic and BigTIFF ARW/NRW containers with root and nested image IFDs;
- Sony `DNGPrivateData`/SR2 metadata decryption and private-IFD parsing;
- uncompressed 12-bit little-endian and 16-bit little-endian samples;
- ARW1 and ARW2 compression `32767`;
- ARW6 compression `32766`;
- single-component tiled lossless-JPEG compression `7`, including the
  two-output-rows-per-JPEG-row Sony packing;
- `ARW` and `NRW` extension routing through `RawFormat`, the facade decoder,
  and CLI discovery;
- mapping image geometry, CFA, black/white levels, white balance, Make/Model,
  and crop data into `fornax_tiff::Directory`;
- typed rejection of unsupported ARQ, reduced-resolution/YCbCr, multi-sample,
  and malformed layouts.

Out of scope:

- pixel-shift ARQ reconstruction;
- reduced-resolution YCbCr `LinearRaw` output;
- non-Bayer CFA layouts;
- camera-specific color matrices that are not present in the source file;
- copying or adapting LGPL decoder source.

## Task checklist

- [x] Add checked TIFF support for parsing an absolute inner IFD and applying
  an offset correction to out-of-line values in decrypted private data.
- [x] Add the `fornax-arw` crate with container selection, Sony decryption,
  private metadata mapping, codec dispatch, active-area crop, and stage
  metadata.
- [x] Use `flashraw-codecs` Sony ARW1/ARW2/ARW6 and lossless-JPEG primitives;
  keep these algorithms format-neutral rather than duplicating them.
- [x] Add `Arw` and `Nrw` variants to the facade decoder and route both through
  `fornax-arw`; preserve DNG and DNC behavior.
- [x] Add synthetic tests for private offsets, packed rows, ARW1 accumulation,
  ARW2 curve/interleaving, Sony LJPEG row unpacking, metadata mapping, CFA
  crop, and unsupported variants.
- [x] Add fixture-backed tests that skip selected raw.pixls.us payloads when
  they are absent, for ARW1, ARW2, and modern compressed samples.
- [x] Document accepted and rejected Sony variants, references, and licensing.
- [x] Update architecture/format status, remove `DEC-ARW` from `TODO.md`, run
  validation, inspect the diff, commit, push, and open one pull request.

## Files expected to change

- `Cargo.toml`
- `TODO.md`
- `crates/fornax-arw/Cargo.toml`
- `crates/fornax-arw/README.md`
- `crates/fornax-arw/src/lib.rs`
- `crates/fornax-arw/src/container.rs`
- `crates/fornax-arw/src/decode.rs`
- `crates/fornax-arw/src/metadata.rs`
- `crates/fornax-arw/src/sony.rs`
- `crates/fornax-arw/src/tests.rs`
- `crates/fornax-raw-core/src/format.rs`
- `crates/fornax-decoder/Cargo.toml`
- `crates/fornax-decoder/src/lib.rs`
- `crates/fornax-tiff/README.md`
- `crates/fornax-tiff/src/lib.rs`
- `crates/fornax-tiff/src/parser.rs`
- `crates/fornax-tiff/tests/private_ifd.rs`
- `crates/fornax-raw/src/lib.rs`
- `docs/architecture.md`
- `docs/decoder/architecture.md`
- `docs/decoder/formats/sony-arw.md`
- `docs/references/sony-arw.md`

## Validation

- `cargo test -p fornax-tiff -p fornax-arw -p fornax-decoder`
- fixture-backed Sony tests with
  `FORNAX_SONY_ARW_FIXTURE`,
  `FORNAX_SONY_ARW2_FIXTURE`, and
  `FORNAX_SONY_LJPEG_FIXTURE`,
  `FORNAX_SONY_UNCOMPRESSED_FIXTURE`, and
  `FORNAX_SONY_ARW6_FIXTURE`;
- `scripts/agent-validation.sh --fix`
- `git diff --check` and a reviewed diff before commit.

Recorded outcomes:

- targeted workspace tests: 32 passed before the final fixture pass;
- five selected Sony fixtures: ARW1, ARW2, uncompressed, and LJPEG samples
  match Rawler sample-for-sample; ARW6 decodes through the native codec and
  passes active-area handoff;
- decoder tracker CI evaluated all 206 ARW corpus rows after the workflow was
  updated to pass `formats=ARW` through to each shard; the updated baseline adds
  201 rows, with 186 ARW passes and 20 typed unsupported/invalid failures;
- generated ARW-to-DNG and DNG-to-TIFF CLI conversions completed successfully;
- `scripts/agent-validation.sh` completed successfully after updating its stale
  benchmark package name from `bench-decoder` to `fornax-decoder`.

## Decisions and notes

- `flashraw-codecs` is already a workspace dependency and is
  `MIT OR Apache-2.0`. Its Sony codecs are the production implementation for
  the bitstream layer; Rawler and LibRaw remain behavioral references only.
- Uncompressed Sony data is little-endian even when the outer TIFF is
  big-endian. The 12-bit path therefore needs the LSB byte packing order rather
  than the DNG big-endian 12-bit convention.
- ARW2 applies the camera curve and emits 14-bit samples. The resulting white
  level must come from Sony private metadata rather than the stored 8-bit
  container depth.
- Tiled Sony LJPEG with four decoded components stores two Bayer output rows
  per JPEG row: R/G from components 0/1 and G/B from components 2/3.
- Reduced-resolution YCbCr, multi-sample ARQ, and unknown compression families
  return `DecodeError::Unsupported`; contradictory offsets or truncated data
  return `DecodeError::InvalidInput`.
- The ILCE-7M4, ILCE-7CR, ILCE-9M3, ILCE-1M2, and DSC-RX1RM3 corpus failures
  are all three-component 15-bit YCbCr reduced RAW modes. They remain expected
  unsupported failures; no synthetic Bayer conversion or linear-RGB pipeline
  expansion is part of this task.
