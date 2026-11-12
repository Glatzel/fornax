# Move DNG-Owned Constants Implementation Plan

This archived workspace-refactor record documents Move DNG-Owned Constants Implementation Plan.

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> `superpowers:executing-plans` to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move only DNG-owned tag/value constants from `fornax-tiff` into
`fornax-dng`, preserve TIFF 6.0, TIFF/EP, and TIFF extension ownership in
`fornax-tiff`, and prove that known DNG values, open tag IDs, and unknown TIFF
field types retain their current parsing and rewriting behavior.

**Architecture:** `fornax-tiff` keeps the dynamic `Tag` key and all TIFF-family
tag/value constants. `fornax-dng` owns DNG-specific compression and photometric
values plus `DngTag`. DNG compression resolution tries DNG/compatibility values
first and then falls back to the Step 5 TIFF resolver. `RasterLayout`,
`RasterRequest`, and `resolve_tiff_compression` remain in `fornax-tiff`.

**Spec:** `docs/_archive/workspace-refactor/dng-constants-step.md`

## Global Constraints

- Implement only Step 6. Do not move vendor codecs, merge format crates, split
  the workspace, rename crates, or publish anything.
- Work from current `origin/main` commit
  `e972b78ac03a8a9861e2909f23d240277e17e388`.
- Do not modify `TODO.md`.
- Preserve dynamic `Tag` keys and the supported TIFF `Value` representation.
  Do not add an open value representation.
- Preserve DNG metadata behavior, decode sample types, writer APIs, decoder
  APIs, and golden decode hashes.
- Keep the Step 5 TIFF raster ownership boundary intact.
- Use TDD for moved constants, conversions, resolver behavior, and writer
  format boundaries. Characterization tests for preservation-only behavior must
  pass on the baseline and remain green after the migration.

## Prerequisite Evidence

- PR #684, `refactor: share generic TIFF accessors`, is merged at
  `86beac28b6c2648e7f95bb831f93fd648a1855b5`.
- PR #686, `refactor: add neutral codec sample buffer`, is merged at
  `00045882e6ea86ad34cdb3c14814b507d060aa97`.
- PR #687, `refactor: add TIFF raster layout support`, is merged at
  `e972b78ac03a8a9861e2909f23d240277e17e388`, the current `origin/main`.
- No Step 6 implementation, plan, or branch existed on `origin/main` before
  this worktree was created.

## Ownership Gate

The gate was completed before branch creation against the pinned TIFF 6.0,
TIFF/EP ISO 12234-2, TIFF Technical Notes 1/2, DNG 1.7.1.0, LibTIFF 4.0.10,
GDAL, and Sony references. No owner remains ambiguous.

### Tag constants

| ID | Meaning and normative source | Domain/type | Current behavior | Owner | Action |
| ---: | --- | --- | --- | --- | --- |
| `254` | `NewSubfileType`, TIFF 6.0 p.36; DNG restrictions pp.20-23 | LONG flags | DNG/ARW raw selection; writer emits `0` | `fornax-tiff` | Keep in TIFF 6.0 group |
| `256` | `ImageWidth`, TIFF 6.0 p.34 | SHORT/LONG scalar | Geometry validation, traversal, writer | `fornax-tiff` | Keep in TIFF 6.0 group |
| `257` | `ImageLength`, TIFF 6.0 p.34 | SHORT/LONG scalar | Geometry validation, traversal, writer | `fornax-tiff` | Keep in TIFF 6.0 group |
| `258` | `BitsPerSample`, TIFF 6.0 | SHORT per sample | Depth selection and writer | `fornax-tiff` | Keep in TIFF 6.0 group |
| `259` | `Compression`, TIFF 6.0 p.30; DNG extension policy pp.20-22 | SHORT scalar | DNG resolves values; writer emits IDs | `fornax-tiff` tag | Keep tag; split value enum |
| `262` | `PhotometricInterpretation`, TIFF 6.0 pp.37-38; DNG pp.22-23 | SHORT scalar | DNG/ARW raw-IFD selection; writer emits CFA | `fornax-tiff` tag | Keep tag; split value enum |
| `271` | `Make`, TIFF 6.0 p.35 | ASCII | ARW manufacturer check | `fornax-tiff` | Keep in TIFF 6.0 group |
| `272` | `Model`, TIFF 6.0 p.35 | ASCII | ARW camera-model fallback | `fornax-tiff` | Keep in TIFF 6.0 group |
| `273` | `StripOffsets`, TIFF 6.0 p.40 | SHORT/LONG array | DNG/ARW strip planning | `fornax-tiff` | Keep in TIFF 6.0 group |
| `274` | `Orientation`, TIFF 6.0 p.36 | SHORT scalar | Preserved metadata | `fornax-tiff` | Keep in TIFF 6.0 group |
| `277` | `SamplesPerPixel`, TIFF 6.0 p.39 | SHORT scalar | Mono RAW enforcement; writer emits `1` | `fornax-tiff` | Keep in TIFF 6.0 group |
| `278` | `RowsPerStrip`, TIFF 6.0 p.39 | SHORT/LONG scalar | Strip geometry and writer | `fornax-tiff` | Keep in TIFF 6.0 group |
| `279` | `StripByteCounts`, TIFF 6.0 p.40 | SHORT/LONG array | Strip validation and writer | `fornax-tiff` | Keep in TIFF 6.0 group |
| `284` | `PlanarConfiguration`, TIFF 6.0 p.38 | SHORT scalar | Writer emits chunky `1` | `fornax-tiff` | Keep in TIFF 6.0 group |
| `317` | `Predictor`, TIFF 6.0 Section 14 p.64 | SHORT scalar | Predictor selection and writer | `fornax-tiff` | Keep in TIFF 6.0 group |
| `322` | `TileWidth`, TIFF 6.0 p.67 | SHORT/LONG scalar | Tile geometry and writer | `fornax-tiff` | Keep in TIFF 6.0 group |
| `323` | `TileLength`, TIFF 6.0 p.67 | SHORT/LONG scalar | Tile geometry and writer | `fornax-tiff` | Keep in TIFF 6.0 group |
| `324` | `TileOffsets`, TIFF 6.0 p.68 | LONG array | Tile validation and writer | `fornax-tiff` | Keep in TIFF 6.0 group |
| `325` | `TileByteCounts`, TIFF 6.0 p.68 | SHORT/LONG array | Tile validation and writer | `fornax-tiff` | Keep in TIFF 6.0 group |
| `330` | `SubIFDs`, TIFF Technical Note 1; TIFF/EP Table 1 and §5.2.5 | LONG IFD-offset array | Parser follows child IFDs | `fornax-tiff` | Keep; remove parser-local numeric duplicate |
| `339` | `SampleFormat`, TIFF 6.0 Section 19 | SHORT per sample | Integer/float decode and writer | `fornax-tiff` | Keep in TIFF 6.0 group |
| `33421` | `CFARepeatPatternDim`, TIFF/EP ISO 12234-2 Table 1 | SHORT[2] | DNG/ARW Bayer repeat shape | `fornax-tiff` | Keep in TIFF/EP group |
| `33422` | `CFAPattern`, TIFF/EP ISO 12234-2 Table 1 | BYTE[rows*columns] | DNG/ARW Bayer pattern | `fornax-tiff` | Keep in TIFF/EP group |
| other | Open TIFF tag namespace | Any understood field type | Unknown IDs preserved as `Tag(id)`; unknown field types skipped | `fornax-tiff` | Preserve |

`SubIFDs` is not in TIFF 6.0. It is defined by TIFF Technical Note 1 and cited
by TIFF/EP. It is a generic TIFF extension, not a DNG-owned tag.

### Compression values

| ID | Meaning and normative source | Domain/type | Current behavior | Owner | Action |
| ---: | --- | --- | --- | --- | --- |
| `1` | Uncompressed, TIFF 6.0 p.30 | SHORT/u16 | TIFF fallback; writer emits `1` | `fornax-tiff` | Keep |
| `7` | TIFF/JPEG, Technical Note 2; DNG restricts CFA payloads to lossless Huffman JPEG | SHORT/u16 | TIFF fallback; ARW has separate Sony LJPEG behavior | `fornax-tiff` default | Keep |
| `8` | Adobe Deflate, used by DNG 1.4 p.21 | SHORT/u16 | TIFF fallback; writer emits `8` | `fornax-tiff` default | Keep |
| `34892` | DNG Lossy JPEG, DNG 1.7.1 p.21 | DNG SHORT/u16 | DNG maps to `LossyJpeg` | `fornax-dng` | Move from TIFF enum |
| `50000` | LibTIFF self-assigned Zstandard, v4.0.10; not Adobe-registered | Non-DNG SHORT/u16 | DNG maps to Zstd; writer emits experimentally | `fornax-dng` | Move from TIFF enum |
| `50002` | GDAL self-assigned TIFF JPEG XL, superseded on write by DNG `52546` | Non-DNG SHORT/u16 | DNG read compatibility maps to JPEG XL | `fornax-dng` | Move from TIFF enum |
| `52546` | DNG 1.7 JPEG XL, DNG 1.7.1 p.21 | DNG SHORT/u16 | DNG maps to JPEG XL | `fornax-dng` | Move from TIFF enum |
| `32766` | Sony-private ARW6 | Vendor SHORT/u16 | ARW6 decoder | `fornax-arw` | Keep |
| `32767` | Sony-private ARW1/ARW2 | Vendor SHORT/u16 | ARW1/ARW2 selection | `fornax-arw` | Keep |

### Photometric values

| ID | Meaning and normative source | Domain/type | Current behavior | Owner | Action |
| ---: | --- | --- | --- | --- | --- |
| `0` | WhiteIsZero, TIFF 6.0 p.37 | SHORT/u16 | Typed recognition | `fornax-tiff` | Keep |
| `1` | BlackIsZero, TIFF 6.0 p.37 | SHORT/u16 | Typed recognition | `fornax-tiff` | Keep |
| `2` | RGB, TIFF 6.0 p.37 | SHORT/u16 | Typed recognition | `fornax-tiff` | Keep |
| `3` | Palette, TIFF 6.0 p.37 | SHORT/u16 | Typed recognition | `fornax-tiff` | Keep |
| `4` | Transparency Mask, TIFF 6.0 p.37 | SHORT/u16 | Typed recognition | `fornax-tiff` | Keep |
| `5` | Separated/CMYK, TIFF 6.0 Section 16 | SHORT/u16 | Typed recognition | `fornax-tiff` | Keep |
| `6` | YCbCr, TIFF 6.0 Section 21 | SHORT/u16 | Typed recognition; unsupported RAW layouts rejected | `fornax-tiff` | Keep |
| `8` | CIE L*a*b*, TIFF 6.0 Section 20 | SHORT/u16 | Typed recognition | `fornax-tiff` | Keep |
| `32803` | CFA, TIFF/EP ISO 12234-2 §5.2.13; DNG references this value | SHORT/u16 | DNG/ARW recognize CFA | `fornax-tiff` | Keep as TIFF/EP |
| `34892` | LinearRaw, DNG 1.7.1 pp.22-23 | DNG SHORT/u16 | DNG recognizes non-CFA raw; ARW rejects reduced-resolution LinearRaw | `fornax-dng` | Move from TIFF enum |
| `51177` | Depth, DNG 1.7.1 pp.22-23 | DNG SHORT/u16 | Not decoded; absent | `fornax-dng` | Add DNG-owned constant |
| `52527` | PhotometricMask, DNG 1.7.1 pp.21-23 | DNG SHORT/u16 | Not decoded; absent | `fornax-dng` | Add DNG-owned constant |

### Private and misplaced constants in consumers

| ID | Meaning and source | Current location/behavior | Owner | Action |
| ---: | --- | --- | --- | --- |
| `50740` | DNG `DNGPrivateData`, DNG 1.7.1 p.41 | Local ARW tag; payload drives Sony private IFD parsing | `fornax-dng` tag, `fornax-arw` payload | Add to `DngTag`; update ARW |
| `0x7010` | SonyRawCurve | ARW2 curve | `fornax-arw` | Keep |
| `0x7038` | Sony RAW image size | Active area | `fornax-arw` | Keep |
| `0x7200`, `0x7201`, `0x7221` | Sony private offset/length/key | Private IFD decryption | `fornax-arw` | Keep |
| `0x7300`, `0x7310` | Sony black levels | ARW levels | `fornax-arw` | Keep |
| `0x7303`, `0x7313` | Sony white balance | ARW neutral gains | `fornax-arw` | Keep |
| `0x787f` | Sony white level | ARW levels | `fornax-arw` | Keep |

### Gate sources

- TIFF 6.0, 3 June 1992:
  <https://download.osgeo.org/libtiff/doc/TIFF6.pdf>.
- DNG 1.7.1.0, September 2023:
  <https://download.adobe.com/pub/adobe/dng/DNG_Spec_1_7_1_0.pdf>.
- TIFF/EP ISO 12234-2:2001 public ISO/DIS draft:
  <https://clanmills.com/exiv2/TAG2000-22_DIS12234-2.pdf>.
- TIFF Technical Note 1:
  <http://www.picturel.com/pdf/ttn1.pdf>.
- TIFF Technical Note 2:
  <https://libtiff.gitlab.io/libtiff/specification/technote2.html>.
- LibTIFF 4.0.10:
  <https://libtiff.gitlab.io/libtiff/releases/v4.0.10.html>.
- GDAL JPEG XL identifier discussion:
  <https://www.mail-archive.com/gdal-dev@lists.osgeo.org/msg42055.html>.
- Sony private references: `docs/reference/decoders/sony-arw.md`.

## Task Checklist

### Task 1: Add RED tests for constant ownership and conversions

- [x] Add tests in `crates/fornax-tiff/tests/constants.rs` asserting TIFF
  compression IDs `34892`, `50000`, `50002`, and `52546` resolve to
  `CompressionMethod::Unknown`, and photometric ID `34892` resolves to
  `PhotometricInterpretation::Unknown`.
- [x] Add tests in `crates/fornax-dng` referencing
  `DngCompressionMethod::{LossyJpeg,Zstd,JpegXlLegacy,JpegXl}` and
  `DngPhotometricInterpretation::{LinearRaw,Depth,PhotometricMask}`, with
  numeric and dynamic-tag round trips and unknown preservation.
- [x] Add a test referencing `DngTag::DngPrivateData` and verifying numeric
  `50740` round trips through `DngTag` and `Tag`.
- [x] Run the focused tests and record the expected compile/assertion failures.

### Task 2: Implement the DNG-owned value split

- [x] Remove DNG and self-assigned compression IDs from
  `fornax_tiff::constants::CompressionMethod`.
- [x] Remove `LinearRaw` from
  `fornax_tiff::constants::PhotometricInterpretation`; retain CFA as a TIFF/EP
  value.
- [x] Add `fornax-dng/src/constants.rs` with DNG-owned compression and
  photometric enums, including `Unknown` catch-alls.
- [x] Add `DngTag::DngPrivateData`.
- [x] Export the new DNG constants module and preserve `fornax_dng::DngTag`.
- [x] Run the focused tests and record GREEN output.

### Task 3: Migrate DNG, ARW, writer, tracker, CLI, and tests

- [x] Update the DNG resolver to use `DngCompressionMethod` before falling back
  to `resolve_tiff_compression`.
- [x] Update DNG raw-IFD detection and writer photometric emission to combine
  TIFF/EP CFA with DNG LinearRaw ownership.
- [x] Update DNG writer compression emission to use the owning TIFF or DNG
  constant while preserving exact bytes.
- [x] Update ARW metadata and tests to import DNG-owned values and
  `DngTag::DngPrivateData`.
- [x] Update all imports in decoder-tracker, CLI, tests, examples, and
  documentation.
- [x] Add a writer-boundary test asserting emitted compression IDs remain
  `1`, `7`, `8`, and `50000` for the corresponding modes.
- [x] Run each affected crate's tests and record GREEN output.

### Task 4: Prove open tags and unknown field types are preserved

- [x] Add a DNG directory round-trip test that writes known DNG tags and an
  unknown dynamic tag ID through `TiffWriter`, parses it, and proves the known
  value resolves through `DngTag` while the unknown ID remains unchanged.
- [x] Add a parser test with an unsupported TIFF field type followed by a
  supported known tag; assert the unsupported field is skipped and later
  entries still parse, matching TIFF 6.0's unknown-field rule.
- [x] Run `cargo test -p fornax-tiff` and `cargo test -p fornax-dng`.

### Task 5: Update ownership documentation

- [x] Update `crates/fornax-tiff/README.md` to remove DNG/self-assigned
  compression variants from the TIFF-owned table.
- [x] Update `crates/fornax-dng/README.md` and `docs/architecture/overview.md` with the
  DNG value ownership boundary.
- [x] Update `docs/architecture/decoder.md` where it states DNG value
  ownership.
- [x] Update `docs/reference/decoders/dng-compression.md` only if the implementation
  changes an ownership statement; preserve its complete compression table.

### Task 6: Validate, inspect, commit, push, and open one PR

- [x] Run all Step 6 focused tests.
- [x] Run workspace tests and `scripts/agent-validation.sh`.
- [x] Run the golden decoder hash check.
- [x] Run `git diff --check`.
- [x] Inspect the complete diff and confirm no Step 5 raster regression.
- [x] Commit the plan, tests, implementation, and documentation.
- [x] Push `chore/dng-owned-constants` and open one semantic PR.
- [x] Stop after reporting the PR URL and validation results.

## Files Expected to Change

- `crates/fornax-tiff/src/constants.rs`
- `crates/fornax-tiff/src/parser.rs`
- `crates/fornax-tiff/tests/constants.rs`
- `crates/fornax-tiff/tests/directory_writer.rs`
- `crates/fornax-tiff/README.md`
- `crates/fornax-dng/src/constants.rs`
- `crates/fornax-dng/src/lib.rs`
- `crates/fornax-dng/src/metadata.rs`
- `crates/fornax-dng/src/dng_decoder/mod.rs`
- `crates/fornax-dng/src/writer.rs`
- `crates/fornax-dng/tests/directory_metadata.rs`
- `crates/fornax-dng/README.md`
- `crates/fornax-arw/src/container.rs`
- `crates/fornax-arw/src/metadata.rs`
- `crates/fornax-arw/src/tests.rs`
- `crates/fornax-cli/tests/cli.rs`
- `crates/decoder-tracker/src/stages.rs`
- `crates/decoder-tracker/src/tests.rs`
- `crates/decoder-tracker/tests/golden.rs`
- `docs/architecture/overview.md`
- `docs/architecture/decoder.md`
- `docs/reference/decoders/dng-compression.md` if ownership prose changes
- `docs/_archive/workspace-refactor/dng-constants-implementation.md`

## Validation

```bash
cargo test -p fornax-tiff
cargo test -p fornax-dng
cargo test -p fornax-arw
cargo test -p decoder-tracker
cargo test -p fornax-cli
cargo test --workspace --all-features --exclude compare-demosaic
scripts/agent-validation.sh
cargo run -q -p decoder-tracker -- hash \
  --manifest crates/decoder-tracker/golden/selected-manifest.csv \
  --check crates/decoder-tracker/golden/decode-hashes.csv
git diff --check
```

## Validation Results

- Focused crate tests: `cargo test -p fornax-tiff -p fornax-dng -p fornax-arw
  -p decoder-tracker -p fornax-cli` passed, 158 tests across 24 suites.
- Workspace tests: `cargo test --workspace --all-features --exclude
  compare-demosaic` passed, 334 tests across 47 suites.
- Golden decode hashes: all 9 selected rows were unchanged. The committed
  corpus gitlink remains unfetchable, so the check used the existing matching
  raw.pixls.us checkout through `--corpus`.
- `scripts/agent-validation.sh` passed after initializing its fixture
  submodules.
- `cargo +nightly fmt --check` and `git diff --check` passed.
- Fresh-context review found one stale ownership sentence in the compression
  reference; it was corrected. The fix pass also added the unknown photometric
  reverse round trip and writer CFA photometric assertions.
- Post-fix checks passed: focused DNG tests 4/4, workspace tests 334/334,
  `RUSTDOCFLAGS="-D warnings" cargo doc -p fornax-dng --all-features
  --no-deps`, nightly formatting, and diff checks.

## Decisions and Notes

- Ruling: `32803` CFA remains owned by TIFF/EP in `fornax-tiff` because DNG
  1.7.1 explicitly says the value is documented by TIFF/EP. Cost if wrong:
  CFA would be placed in the wrong crate despite the normative source.
- Ruling: `SubIFDs` remains a generic TIFF extension owned by `fornax-tiff`
  because it is defined by TIFF Technical Note 1 and incorporated by TIFF/EP.
  Cost if wrong: generic TIFF tree parsing would incorrectly depend on DNG.
- Ruling: `50000` and `50002` are DNG compatibility constants because only the
  DNG decoder/writer path interprets them, although neither is Adobe DNG
  defined. Cost if wrong: the DNG compatibility path would lose explicit
  ownership of its non-standard IDs.
- Ruling: open tag IDs remain dynamic `Tag(u16)` keys and unknown field types
  continue to be skipped. No open `Value` representation is introduced.
  Cost if wrong: unknown metadata could be represented incorrectly or writer
  behavior could change beyond Step 6.
- The TIFF resolver remains limited to `1`, `7`, and `8`, preserving the
  Step 5 ownership boundary.
- The DNG writer uses typed TIFF/DNG constants but emits the same numeric
  compression IDs as before, so serialized behavior is unchanged.
