# Panasonic RW2 Decoder

This archived plan documents the completed Panasonic RW2 Decoder work.

## Goal

Add native pure-Rust decoding for single-shot Panasonic RW2 files using
conventional 2x2 RGB Bayer storage. The supported raw-format families are V4,
V5, V6, and V8, plus stamped 12-bit storage selected by payload length.
Return typed `DecodeError::Unsupported` errors without conversion fallback for
V7, multi-exposure/pixel-shift files, unknown versions, unknown compression,
legacy `.RAW`, unsupported CFA layouts, and non-rectangular layouts.

## Scope

In scope:

- Accept classic-TIFF magic 85 in `fornax-tiff`.
- Add a feature-gated `rw2` codec family to `fornax-codec`.
- Add a `fornax-rw2` container crate implementing the shared decoder and DNG
  decoder contracts.
- Route `.RW2` through the native decoder in `fornax-decoder`.
- Map geometry, crop, CFA, black/white levels, neutral, orientation, serial,
  lens, and DNG-compatible metadata.
- Add synthetic corruption tests, real-fixture parity tests, golden decode
  hashes, expected-unsupported corpus cases, and format/reference
  documentation.
- Remove DEC-RW2 from `TODO.md` after validation succeeds.

Out of scope:

- V7 codecs and V7 files. A codec will not be written until a real single-shot
  V7 fixture is available for validation.
- Legacy Panasonic `.RAW`, raw-format versions 1-3, multi-exposure,
  pixel-shift, and hand-held high-resolution composite layouts.
- Camera color matrices that are not present in the source metadata.
- Any production dependency on Rawler, LibRaw, or another LGPL decoder.

## Pinned Fixture Decisions

The four supported corpus fixtures and three expected-unsupported fixtures
were inspected directly before implementation:

| Case | Version `0x002d` | Compression `0x000b` | CFA `0x0009` | Border origin |
| --- | ---: | ---: | ---: | --- |
| `DMC-G85/_1030982.RW2` | 4 | 34316 | 4 (BGGR) | `(8, 8)` |
| `DC-GH5S/P1022085.RW2` | 5 | 34316 | 1 (RGGB) | `(12, 8)` |
| `DC-S1/P1033119.RW2` | 6 | 34316 | 1 (RGGB) | `(12, 8)` |
| `DC-GH6/4-3.RW2` | 8 | absent | 1 (RGGB) | `(8, 8)` |
| `DC-G9/P1000475.RW2` | 5 | 34316 | 1 | `(18, 10)` |
| `DC-S5/dc-s5_hr_96MP.RW2` | 7 | 34316 | 1 | `(16, 8)` |
| `DC-S1RM2/S1Rii_multiexposure.RW2` | 8 | absent | 1 | `(4, 4)` |

Dispatch rules:

- The raw-format tag is Panasonic `0x002d`.
- The legacy Panasonic compression tag is `0x000b`. For V4-V6 it must be
  `34316`; V8 accepts an absent compression tag because the supported V8
  fixture omits it, and accepts `34316` if a future file supplies it.
- Versions 4, 5, 6, and 8 select `PanasonicV4`, `PanasonicV5`,
  `PanasonicV6`, and `PanasonicV8`, respectively.
- Unknown or missing versions and unknown compression values are typed
  `Unsupported`.
- A nonzero `0x0121` in the raw IFD, or a nonzero CameraIFD `0x1001`, marks a
  multi-shot file and returns `Unsupported` before sample decoding. The
  fallback is needed for G9 and S5 high-resolution files whose raw-format
  values otherwise resemble supported single-shot files.
- V7 always returns `Unsupported`, even for a single-shot candidate, until a
  real single-shot V7 file can validate the codec.

Metadata decisions:

- Border tags `0x0004`-`0x0007` are top, left, bottom, and right coordinates;
  the right/bottom values are exclusive.
- Crop tags `0x002f`-`0x0032` are top, left, bottom, and right coordinates,
  not width/height margins. For example, V4 stores `8, 8, 3456, 4600`,
  producing a `4592x3448` crop at `(8, 8)`. DNG default crop origin is
  relative to `ActiveArea`; bottom/right are exclusive.
- CFA value `1` through `4` maps to RGGB, GRBG, GBRG, and BGGR at full-sensor
  coordinates. Shift the pattern by the `ActiveArea` origin modulo two when
  forming DNG metadata.
- Black component tags are `0x001c`, `0x001d`, and `0x001e`. Add 15 when
  `0x0118` is absent or the version is at most 4.
- Linearity limits are `0x000e` (red), `0x000f` (green), and `0x0010` (blue).
  Reduce present, nonzero values with `max`; use the default
  `(1 << bits) - 1` only when all three are absent or zero. Partial tags are
  valid and use the maximum present component.
- Neutral uses `0x0011`/`0x0012` when both are present, otherwise
  `0x0024`/`0x0025`/`0x0026`. The green component is fixed at 256 for the
  first pair and read from `0x0025` for the second pair. Normalize green to
  one.
- Orientation comes from raw-IFD TIFF tag `0x0112`. Camera serial number
  (`0xa431`) and lens model (`0xa434`) come from the Exif IFD reachable from
  `0x8769`; each is optional. No makernote parser is required.

## Safety Rules

- Reject layouts whose declared dimensions exceed 64,000,000 pixels as
  high-resolution composite candidates. Normal single-shot Panasonic sensors
  remain below this bound.
- Use checked arithmetic for sample counts, row sizes, bit ranges, strip
  offsets/sizes, and allocation lengths.
- Validate payload/chunk bounds against declared dimensions before allocating
  the output mosaic.
- V8 parameter and strip tables validate counts, dimensions, offsets, Huffman
  symbols, and output coverage in constructors and again during decode.
- Codec truncation tests cover every prefix length of representative inputs.
  Byte-flip mutation tests cover representative V4, V5, V6, V8, and stamped
  payloads and require an error or valid bounded decode, never a panic.

## Task Checklist

- [x] Add and test classic-TIFF magic 85 parsing without weakening BigTIFF or
  malformed-header rejection.
- [x] Add `fornax-codec` `rw2` feature and independent Panasonic compression
  variants.
- [x] Write synthetic V4 tests, then implement V4 block/split handling.
- [x] Write synthetic V5 tests, then implement V5 packet decoding.
- [x] Write synthetic V6 tests, then implement V6 packet/row decoding.
- [x] Write validating `PanasonicV8Params`/`PanasonicV8Strip` tests, then
  implement V8 Huffman, gamma, strip placement, and identity/nonlinear paths.
- [x] Write stamped 12-bit tests, then implement unpacked and packed-control
  layouts.
- [x] Add truncation and byte-flip mutation coverage for every RW2 codec.
- [x] Add `fornax-rw2` container parsing, layout selection, version/compression
  dispatch, multi-shot rejection, CFA/geometry/crop/level/neutral mapping, and
  DNG round-trip tests.
- [x] Implement `Rw2Image` accessor surface and `IDecoder`/`IDecoderDng`.
- [x] Add `Decoder::Rw2` routing without changing existing format routing.
- [x] Validate the four supported fixtures against committed Rawler reference
  digests without linking Rawler into production targets.
- [x] Add supported RW2 rows to the golden manifest and generated decode-hash
  file, and add expected-unsupported tests for the three named files.
- [x] Run the RW2 corpus workflow/selection, review baseline changes, regenerate
  the tracker README, and require a clean rerun.
- [x] Write Panasonic format/reference docs, update architecture and crate
  READMEs, remove DEC-RW2 from `TODO.md`, and run full validation.

## Files Expected to Change

- `crates/fornax-tiff/src/parser.rs`
- `crates/fornax-tiff/tests/`
- `crates/fornax-codec/Cargo.toml`
- `crates/fornax-codec/README.md`
- `crates/fornax-codec/src/{lib.rs,request.rs,error.rs}`
- `crates/fornax-codec/src/rw2/`
- `crates/fornax-codec/tests/rw2.rs`
- `crates/fornax-rw2/`
- `crates/fornax-decoder/{Cargo.toml,README.md}`
- `crates/fornax-decoder/src/lib.rs`
- `crates/decoder-tracker/golden/`
- `crates/decoder-tracker/{baseline.csv,README.md}`
- `crates/decoder-tracker/src/` and tests only where RW2 result classification
  needs regression coverage.
- `docs/architecture/overview.md`
- `docs/decoder/formats/panasonic-rw2.md`
- `docs/reference/decoders/panasonic-rw2.md`
- `README.md` and affected crate READMEs
- `Cargo.toml`, `Cargo.lock`
- `TODO.md`

## Detailed Tasks

### Task 1: TIFF Magic 85

Commit: `feat: accept Panasonic classic TIFF magic`.

1. Add a failing test proving magic 85 parses a classic IFD.
2. Add a failing test proving magic 85 with a BigTIFF-looking header remains
   a classic layout and does not enable BigTIFF parsing.
3. Preserve existing malformed-header, BigTIFF, and unsupported-layout tests.
4. Accept magic 85 only as classic TIFF.
5. Run `cargo test -p fornax-tiff`.

### Task 2: RW2 Codecs

Commit: `feat: add Panasonic RW2 codecs`.

1. Add the independent `rw2` feature and red/green contract tests for
   `PanasonicV4`, `PanasonicV5`, `PanasonicV6`, `PanasonicV8`,
   `PanasonicUnpacked12`, and `PanasonicPacked12Control`.
2. Implement V4 using checked 14-sample groups and optional split blocks.
3. Implement V5 using validated 0x4000-byte blocks and 12/14-bit packets.
4. Implement V6 using validated 16-byte row packets and 12/14-bit layouts.
5. Add validating, `#[non_exhaustive]` V8 parameter/strip constructors, then
   implement Huffman decode, identity/nonlinear gamma, and strip placement.
6. Add exact stamped-12-byte classification and both storage decoders.
7. Add invalid geometry, overflow, malformed-table, truncation sweep, and
   byte-flip mutation tests.
8. Run the focused codec tests with `--features rw2` and `--all-features`.

### Task 3: RW2 Container

Commit: `feat: add native RW2 container decoding`.

1. Add failing tests for offset-versus-strip selection, version dispatch,
   compression dispatch, multi-shot rejection, V7 rejection, and unknown
   values.
2. Add checked container/geometry parsing with the 64,000,000-pixel
   single-shot cap.
3. Add failing metadata tests and mappings for border/active area, crop,
   CFA shifting, black/white levels, neutral, orientation, serial, and lens.
4. Implement `Rw2Image` with the RAF-style accessors and trait impls.
5. Add a DNG write/read round-trip test.
6. Run `cargo test -p fornax-rw2 --all-features`.

### Task 4: Decoder Facade

Commit: `feat: route RW2 through the native decoder`.

1. Add a failing routing test for `.RW2`.
2. Add unconditional `fornax-rw2` dependency and `Decoder::Rw2`.
3. Forward all `IDecoder` and `IDecoderDng` methods without changing existing
   routes.
4. Run `cargo test -p fornax-decoder --all-features`.

### Task 5: Corpus and Reference Parity

Commit: `test: add RW2 corpus coverage`.

1. Generate Rawler reference digests with an out-of-workspace tool and commit
   the expected values.
2. Verify dimensions, every sample, levels, neutral, and crop for the four
   supported fixtures.
3. Add the four passing fixtures to the golden manifest and regenerate decode
   hashes.
4. Add expected-unsupported regression cases for the G9, S5 high-resolution,
   and S1R2 multi-exposure fixtures.
5. Run the RW2 tracker selection, review the baseline, regenerate the README,
   and require a clean rerun.

### Task 6: Documentation and Completion

Commit: `docs: document Panasonic RW2 support`.

1. Add the Panasonic format plan, reference note, and crate README.
2. Update root architecture and public README coverage.
3. Record V7 as deferred pending a real single-shot validation file.
4. Remove DEC-RW2 from `TODO.md`.
5. Run all focused commands, `scripts/agent-validation.sh`, and
   `git diff --check`.

## Validation

Run the focused commands throughout and the full command before handoff:

```bash
cargo test -p fornax-tiff
cargo test -p fornax-codec --features rw2
cargo test -p fornax-codec -p fornax-rw2 -p fornax-decoder --all-features
cargo test -p fornax-codec -p fornax-rw2 -p fornax-decoder --all-features --doc
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --check
scripts/agent-validation.sh
git diff --check
```

Manual corpus validation uses `formats='**/*.[rR][wW]2'` with
`run_duckdb_tests` disabled. Report any skipped checks explicitly.

## Decisions and Notes

- The brief's crop paragraph described primary tags `0x002f`-`0x0032` as
  margins, but all four fixtures store exclusive lower/right coordinates.
  Implementing them as coordinates matches both the tags and reference
  decoders.
- White-level component disagreement is reduced by `max`; this preserves the
  available saturation headroom for a single-level mosaic while allowing
  partial tags.
- Serial and lens metadata are preserved when Exif supplies `0xa431` and
  `0xa434`; they are not required for decode success. Orientation is preserved
  from the raw IFD.
- Rawler 0.8.0 was inspected only as a development reference for Format
  behavior and V8 parameter structure. Fornax code and tests are derived from
  documented bitstream behavior and fixture evidence, not copied source.
- Corpus review showed malformed optional CameraIFDs on older files and
  unmarked 96-100 MP composite layouts. Malformed CameraIFDs are ignored;
  layouts above 64 MP return typed unsupported errors.
- The S1R2 multi-exposure fixture has no documented multishot marker, so its
  pinned `[4144, 4008]` V8 strip split is also rejected structurally.
- The reviewed RW2 corpus baseline contains 384 passing files and 18 typed
  unsupported composite/pixel-shift/V7 files. The clean rerun succeeded and
  uploaded no baseline-update artifact.
- Older DMC-G7/G70 files store all-zero crop tags as placeholders; the loader
  maps those to the full active area.
