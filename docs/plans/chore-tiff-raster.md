# TIFF Raster Layout Support Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move generic TIFF strip/tile raster planning and TIFF-registered
compression resolution into `fornax-tiff`, while DNG retains its format-specific
compression overrides and all decoder outputs remain unchanged.

**Architecture:** `fornax-tiff` will expose a validated `RasterLayout` parsed
from a `Directory`, a `RasterRequest` that accepts an already-resolved
`fornax_codec::DecodeCompression`, and a default resolver for TIFF
compression IDs `1`, `7`, and `8`. `fornax-dng` will resolve DNG and RAW
compatibility IDs first, then fall back to the TIFF resolver, and will build the
shared request before dispatching to `fornax-codec`. `fornax-arw` and
`fornax-codec` retain vendor and codec behavior respectively.

**Tech Stack:** Rust 2024, `fornax-tiff`, `fornax-codec`, `fornax-dng`,
`fornax-arw`, Cargo tests, rustdoc, Clippy, and repository validation scripts.

**Spec:** `docs/plans/workspace-refactor/step-05-tiff-raster.md`

## Global Constraints

- Work from current `origin/main` commit `0004588`.
- Implement only Step 5. Do not begin Step 6.
- Do not modify `TODO.md`.
- Do not make `fornax-tiff` depend on DNG.
- Do not move `active_area`, `default_crop`, CFA semantics, or other DNG or
  format policy into `fornax-tiff`.
- Do not move DNG-owned constants. Those stay for Step 6.
- Do not move Sony vendor codecs or reinterpret Sony-private compression IDs.
- Preserve strip/tile row-major ordering, tile dimensions, rows per strip,
  partial final chunks, offset and byte-count validation, predictor behavior,
  existing error categories and useful error detail.
- Preserve the Step 4 `SampleBuffer` / `RawMosaic` adapter boundary.
- Preserve decoder and writer source compatibility and golden decode hashes.
- Use TDD for every new raster API, resolver, validation path, and format
  boundary behavior. Watch each new test fail for the expected missing behavior
  before implementing it.

## Compression Ownership Gate

The gate was completed before branch creation against the pinned TIFF, DNG,
LibTIFF, GDAL, and Sony references. No ownership is unresolved.

| ID | Normative meaning and source | Supported data/layout | Current behavior | Ownership boundary |
| ---: | --- | --- | --- | --- |
| `1` | TIFF 6.0 uncompressed, tightly packed; TIFF 6.0 section 8 and Compression field on p.30 | Fornax mono integer 1-32-bit or float 16/24/32-bit; strips or tiles | DNG maps to `Uncompressed`; DNG writer emits `1` | TIFF default resolver to codec `Uncompressed` |
| `7` | TIFF Technical Note #2 complete ISO JPEG segment, sequential lossy or lossless; DNG 1.7.1 pp.20-21 requires lossless Huffman JPEG for CFA raw payloads | Fornax mono lossless JPEG-92, 2-16-bit; ARW has a vendor-specific LJPEG tile layout | DNG maps to `LosslessJpeg`; ARW maps to Sony LJPEG; DNG writer emits `7` | TIFF default resolver to `LosslessJpeg` for the supported mono raw subset; DNG keeps applicability checks; ARW keeps vendor semantics |
| `8` | Adobe Deflate extension; DNG 1.7.1 pp.20-21 defines Deflate (ZIP) and permitted DNG storage types; LibTIFF names it `COMPRESSION_ADOBE_DEFLATE` | Integer and float payloads with TIFF predictors; strips or tiles | DNG maps to `Deflate`; DNG writer emits `8` | TIFF default resolver to codec `Deflate`; DNG retains version and sample-type policy |
| `32766` | Sony-private ARW6 compression with no TIFF registration; pinned dnglab/Rawler and LibRaw behavior in `docs/references/sony-arw.md` | Sony wavelet stream decoded to one `u16` mosaic | ARW maps to `Arw6` | `fornax-arw` only |
| `32767` | Sony-private ARW1/ARW2 compression with no TIFF registration; pinned dnglab/Rawler and LibRaw behavior in `docs/references/sony-arw.md` | One-strip packed Sony stream; stored byte count distinguishes ARW2 | ARW maps to `Arw1` or `Arw2` | `fornax-arw` only |
| `34892` | DNG 1.4+ Lossy JPEG; DNG 1.7.1 p.21 and Appendix A | DNG 8-bit `LinearRaw` or `PhotometricMask`; Fornax supports one 8-bit component | DNG maps to `LossyJpeg` | DNG override to codec `LossyJpeg` |
| `50000` | LibTIFF self-assigned Zstandard, explicitly not Adobe-registered; LibTIFF 4.0.10 release notes | Fornax integer or float raw payloads | DNG maps to `Zstd`; DNG writer emits `50000` experimentally | DNG compatibility override to codec `Zstd` |
| `50002` | GDAL self-assigned JPEG XL TIFF value, not Adobe-registered and superseded for writing by DNG `52546`; GDAL discussion and current LibTIFF header warning | Fornax single-channel 8-16-bit integer JPEG XL | DNG maps to `JpegXl` | DNG compatibility override to codec `JpegXl` |
| `52546` | DNG 1.7 JPEG XL; DNG 1.7.1 pp.21 and Appendix A | DNG permits 8-16-bit integer, 16-bit float, 1 or 3 planes; Fornax supports one 8-16-bit integer plane | DNG maps to `JpegXl` | DNG override to codec `JpegXl` |

TIFF defaults are therefore only `1`, `7`, and `8`. DNG resolves `34892`,
`50000`, `50002`, and `52546` before falling back. TIFF `7` baseline-DCT is
outside the supported single-band RAW subset; this is a documented layout
limitation, not an ownership ambiguity.

## Review Focus

- A tiled image whose width or height is not a multiple of the tile size must
  decode only the in-image portion of the final row or column without shifting
  earlier chunks.
- Strip and tile chunk lists must remain in tag order, which is row-major
  geometry order, even when file offsets are non-monotonic.
- A chunk whose `offset + byte_count` overflows or exceeds the source must fail
  as invalid input before the codec reads it.
- DNG-specific IDs must resolve before TIFF fallback, while Sony-private IDs
  must never be interpreted by the TIFF resolver.
- The migration must not alter sample storage type, transforms, process
  metadata, DNG metadata hashes, or writer source APIs.

---

### Task 1: Add the Generic TIFF Raster Model and Default Resolver

**Files:**

- Create: `crates/fornax-tiff/src/raster.rs`
- Modify: `crates/fornax-tiff/src/lib.rs`
- Modify: `crates/fornax-tiff/Cargo.toml`
- Create: `crates/fornax-tiff/tests/raster.rs`
- Modify: `crates/fornax-tiff/README.md`

**Interfaces:**

- Consumes: `fornax_codec::{ByteOrder, Chunk, DecodeCompression, DecodeRequest,
  Predictor, SampleFormat}` and existing `fornax_tiff::{Directory, Tag,
  unsigned_scalar, unsigned_vec}`.
- Produces:
  - `pub enum RasterStorage { Strips, Tiles }`
  - `pub struct RasterLayout`
  - `RasterLayout::from_directory(directory: &Directory) -> Result<RasterLayout, RasterError>`
  - `RasterLayout::{storage, width, height, chunk_width, chunk_height, chunks}`
  - `pub struct RasterRequest<'a>`
  - `RasterRequest::new(source: &'a [u8], directory: &Directory,
    compression: DecodeCompression, bits_per_sample: u16,
    sample_format: SampleFormat, byte_order: ByteOrder, predictor: Predictor)
    -> Result<RasterRequest<'a>, RasterError>`
  - `RasterRequest::codec_request(&self) -> DecodeRequest<'_>`
  - `pub fn resolve_tiff_compression(id: u16) -> Option<DecodeCompression>`
  - `pub enum RasterError`

- [ ] **Step 1: Add the TIFF dependency on the neutral codec crate**

Add `fornax-codec.workspace = true` to `crates/fornax-tiff/Cargo.toml`.

- [ ] **Step 2: Write failing strip and tile layout tests**

In `crates/fornax-tiff/tests/raster.rs`, add
`strips_preserve_rows_per_strip_and_chunk_order` and
`tiles_preserve_geometry_and_row_major_chunk_order`. Build directories with
`ImageWidth=5`, `ImageLength=5`, and:

- strips: `RowsPerStrip=2`, offsets `[100, 20, 70]`, counts `[10, 11, 12]`;
- tiles: `TileWidth=3`, `TileLength=2`, offsets `[90, 10, 60, 30, 50, 40]`,
  counts `[1, 2, 3, 4, 5, 6]`.

Assert storage kind, full dimensions, chunk dimensions, chunk count, and exact
`Chunk` values in tag order.

- [ ] **Step 3: Run the new tests and watch them fail**

Run:

```bash
cargo test -p fornax-tiff --test raster strips_preserve_rows_per_strip_and_chunk_order tiles_preserve_geometry_and_row_major_chunk_order
```

Expected: compile failure because `RasterLayout`, `RasterStorage`, and the
`raster` module do not exist.

- [ ] **Step 4: Add the minimal `RasterLayout` implementation**

Implement `RasterStorage`, `RasterLayout`, and `RasterError`. Select tiles when
`TileOffsets` is present, matching current DNG behavior; otherwise read strips.
Use `ImageWidth` and `ImageLength` for full geometry. For strips, chunk width is
the full image width and chunk height is `RowsPerStrip`; for tiles, use
`TileWidth` and `TileLength`. Reject zero dimensions, missing or invalid tags,
offset/count length mismatches, computed chunk-count mismatches, and chunk
range overflow. Preserve offsets and counts in input order.

- [ ] **Step 5: Run the layout tests and watch them pass**

Run the command from Step 3. Expected: both tests pass.

- [ ] **Step 6: Write failing partial-geometry and validation tests**

Add these tests:

- `decode_tiles_with_partial_final_geometry_preserves_row_major_copy`: use a
  `5x3` uncompressed 8-bit image with `4x2` tiles and payload bytes chosen so
  each source sample maps uniquely to its destination position; decode through
  `RasterRequest::codec_request()` and `fornax_codec::decode`, then assert the
  full `U8` sample vector and dimensions.
- `missing_byte_counts_reports_the_layout_tag`: omit `TileByteCounts` and assert
  `RasterError::MissingTag(Tag::from(TiffTag::TileByteCounts))`.
- `mismatched_offsets_and_byte_counts_are_rejected`: provide two offsets and
  one byte count and assert `RasterError::ChunkListLengthMismatch`.
- `chunk_range_overflow_is_rejected`: provide `offset=usize::MAX`,
  `byte_count=1` and assert `RasterError::ChunkRangeOverflow { index: 0 }` when
  constructing `RasterRequest`.
- `chunk_outside_source_is_rejected`: provide an in-range offset whose count
  extends past the source and assert `RasterError::ChunkOutOfBounds`.
- `zero_dimensions_are_rejected_with_the_source_tag`: provide zero
  `TileWidth` and assert `RasterError::ZeroDimension`.

- [ ] **Step 7: Run the new tests and watch them fail**

Run:

```bash
cargo test -p fornax-tiff --test raster
```

Expected: failures for missing `RasterRequest` or missing validation variants,
not syntax errors.

- [ ] **Step 8: Implement `RasterRequest` and validation**

`RasterRequest::new` parses `RasterLayout`, checks every `offset +
byte_count` with checked arithmetic against `source.len()`, and stores the
already-resolved codec compression plus sample layout fields.
`codec_request()` returns a `fornax_codec::DecodeRequest` borrowing the stored
chunk vector. Do not inspect the TIFF `Compression` tag here.

- [ ] **Step 9: Run all TIFF raster tests**

Run `cargo test -p fornax-tiff --test raster`. Expected: all pass.

- [ ] **Step 10: Write failing TIFF-default resolver tests**

Add `tiff_registered_compression_ids_use_tiff_defaults` asserting:

- `1 -> Some(Uncompressed)`
- `7 -> Some(LosslessJpeg)`
- `8 -> Some(Deflate)`
- unknown `12_345 -> None`

Add `non_tiff_compression_ids_do_not_use_tiff_defaults` asserting `None` for
`34892`, `50000`, `50002`, `52546`, `32766`, `32767`.

- [ ] **Step 11: Run resolver tests and watch them fail**

Run:

```bash
cargo test -p fornax-tiff --test raster compression_ids
```

Expected: compile failure because `resolve_tiff_compression` is absent.

- [ ] **Step 12: Implement the default resolver**

Implement `resolve_tiff_compression(id: u16) -> Option<DecodeCompression>` with
only the approved TIFF defaults `1`, `7`, and `8`. Keep DNG, self-assigned, and
vendor IDs out of this function.

- [ ] **Step 13: Run the complete TIFF crate tests**

Run `cargo test -p fornax-tiff`. Expected: all tests pass.

- [ ] **Step 14: Document and commit Task 1**

Document the new model and ownership boundary in `crates/fornax-tiff/README.md`.
Commit:

```bash
git add crates/fornax-tiff/Cargo.toml crates/fornax-tiff/src/lib.rs \
  crates/fornax-tiff/src/raster.rs crates/fornax-tiff/tests/raster.rs \
  crates/fornax-tiff/README.md
git commit -m "refactor: add TIFF raster layout model"
```

### Task 2: Migrate DNG Compression Resolution and Raster Planning

**Files:**

- Modify: `crates/fornax-dng/src/dng_decoder/mod.rs`
- Modify: `crates/fornax-dng/README.md`
- Modify: `docs/references/dng-compression.md`
- Modify: `docs/architecture.md`

**Interfaces:**

- Consumes: `fornax_tiff::{RasterLayout, RasterRequest, RasterError,
  resolve_tiff_compression}` and the Task 1 request constructor.
- Produces: private `resolve_dng_compression(id: u16) -> Result<DecodeCompression, DecodeError>`
  and `decode_plan(...) -> Result<RasterRequest<'_>, DecodeError>`. Public DNG
  decoder APIs remain unchanged.

- [ ] **Step 1: Write failing DNG boundary tests**

Replace or extend the existing decode-plan tests in
`crates/fornax-dng/src/dng_decoder/mod.rs` with:

- `dng_compression_resolution_prefers_dng_ids_and_falls_back_to_tiff`:
  assert `1`, `7`, `8`, `34892`, `50000`, `50002`, and `52546` map to the
  expected codec compressions.
- `unknown_dng_compression_remains_invalid_input`: assert ID `12_345` returns
  `DecodeError::InvalidInput` and detail contains `invalid DNG compression
  12345`.
- `dng_raster_request_preserves_tile_predictor_and_chunks`: adapt the existing
  DNG decode-plan test to inspect `RasterRequest::layout()`, compression,
  predictor, bits, sample format, byte order, and chunks.

Keep the existing legacy JPEG XL and zstd mapping tests.

- [ ] **Step 2: Run the DNG tests and watch them fail**

Run:

```bash
cargo test -p fornax-dng dng_compression_resolution_prefers_dng_ids_and_falls_back_to_tiff unknown_dng_compression_remains_invalid_input dng_raster_request_preserves_tile_predictor_and_chunks
```

Expected: compile failure because DNG still returns its private `DecodePlan`
and has no `resolve_dng_compression` boundary.

- [ ] **Step 3: Implement DNG-specific resolution before TIFF fallback**

Read the compression tag once as `u16`. Map `34892`, `50000`, `50002`, and
`52546` in `fornax-dng`; otherwise call
`fornax_tiff::resolve_tiff_compression(id)`. Convert unknown IDs to the existing
`corrupt("invalid DNG compression {id}")` category. Do not infer TIFF ownership
from `CompressionMethod` variant names.

- [ ] **Step 4: Replace the private layout planner with `RasterRequest`**

Keep DNG policy in `fornax-dng`: image dimensions, sample count, sample format,
byte order, and predictor selection. Delegate strip/tile tag parsing, chunk
list validation, chunk-count checks, and source range checks to
`RasterRequest::new`. Map `RasterError` to `DecodeError::InvalidInput`.
`decode_samples` calls `fornax_codec::decode(&plan.codec_request())` and still
adapts through `into_raw_mosaic`.

- [ ] **Step 5: Run focused DNG tests**

Run `cargo test -p fornax-dng`. Expected: all tests pass, including writer
round trips and transforms.

- [ ] **Step 6: Update ownership documentation**

Add the complete gate table and TIFF-versus-DNG resolution rule to
`docs/references/dng-compression.md`. Update `crates/fornax-dng/README.md` and
`docs/architecture.md` so `fornax-tiff -> fornax-codec` is explicit and DNG is
documented as the override/fallback owner. Do not move constants.

- [ ] **Step 7: Commit Task 2**

Run `cargo test -p fornax-tiff` and `cargo test -p fornax-dng`, then commit:

```bash
git add crates/fornax-dng/src/dng_decoder/mod.rs \
  crates/fornax-dng/README.md docs/references/dng-compression.md \
  docs/architecture.md
git commit -m "refactor: migrate DNG raster planning"
```

### Task 3: Verify Compatibility, Open the PR, and Stop

**Files:**

- Verify all changed files.
- Do not modify `TODO.md`, Step 6 files, ARW codecs, or workspace layout.

**Interfaces:**

- Consumes: completed Task 1 and Task 2 interfaces.
- Produces: one pushed branch and one pull request against `main`.

- [ ] **Step 1: Inspect local layout and compression call sites**

Run:

```bash
rg -n "strip|tile|rows_per_strip|TileOffsets|StripOffsets|CompressionMethod|DecodeCompression" \
  crates/fornax-tiff/src crates/fornax-codec/src crates/fornax-dng/src \
  crates/fornax-arw/src crates/fornax-decoder/src crates/decoder-tracker/src
git diff --stat origin/main
git diff origin/main -- crates/fornax-tiff crates/fornax-dng docs
```

Expected: generic DNG strip/tile planning is gone; ARW vendor layout remains;
no Step 6 constant move or workspace change is present.

- [ ] **Step 2: Run focused tests**

```bash
cargo test -p fornax-tiff
cargo test -p fornax-dng
cargo test -p fornax-arw
cargo test -p decoder-tracker
```

Expected: every command exits 0 with no failures.

- [ ] **Step 3: Run the full workspace tests**

```bash
cargo test --workspace --all-features --exclude compare-demosaic
```

Expected: exit 0 with no failures.

- [ ] **Step 4: Run repository validation**

```bash
scripts/agent-validation.sh
```

Expected: exit 0. Record each failed or skipped check explicitly if not.

- [ ] **Step 5: Run the golden decode hash gate**

```bash
cargo run -q -p decoder-tracker -- hash \
  --manifest crates/decoder-tracker/golden/selected-manifest.csv \
  --check crates/decoder-tracker/golden/decode-hashes.csv
```

Expected: exit 0 with all selected golden hashes unchanged.

- [ ] **Step 6: Run whitespace validation**

```bash
git diff --check
```

Expected: no output and exit 0.

- [ ] **Step 7: Commit the finished tree if needed and push**

If Task 2 did not commit the final documentation changes, commit them now.
Then:

```bash
git status --short
git push -u origin HEAD
```

Expected: clean worktree and a new remote branch `chore/tiff-raster`.

- [ ] **Step 8: Open one PR against `main`**

Title:

```text
refactor: add TIFF raster layout support
```

The body must include:

- the complete compression-ownership table with normative sources;
- the statement that no maintainer ruling was needed;
- the TIFF-versus-DNG resolution boundary;
- TDD coverage added;
- validation commands and results;
- golden-hash result;
- deviations, or `None`.

- [ ] **Step 9: Stop**

Report the PR URL, validation outcomes, golden-hash result, and deviations.
Do not watch, poll, merge, or close the PR.

## Validation

```bash
cargo test -p fornax-tiff
cargo test -p fornax-dng
cargo test -p fornax-arw
cargo test -p decoder-tracker
cargo test --workspace --all-features --exclude compare-demosaic
scripts/agent-validation.sh
cargo run -q -p decoder-tracker -- hash \
  --manifest crates/decoder-tracker/golden/selected-manifest.csv \
  --check crates/decoder-tracker/golden/decode-hashes.csv
git diff --check
```

## Decisions and Notes

- No maintainer ruling is required: every mapped ID has an identified owner and
  normative or vendor-reference meaning.
- TIFF `1`, `7`, and `8` are the only default-resolver IDs.
- DNG overrides `34892`, `50000`, `50002`, and `52546` before TIFF fallback.
- Sony `32766` and `32767` remain wholly in `fornax-arw`.
- The generic raster request accepts resolved codec compression; it does not
  own DNG, self-assigned, or vendor ID policy.
- DNG constants remain in `fornax-tiff` until Step 6.
