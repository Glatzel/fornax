# Nikon NEF reference notes

## Sources

- dnglab/Rawler at
  `62b77de1a206d467e910d55199191068b45a9cc1`, especially
  `rawler/src/decoders/nef.rs`, `rawler/src/decoders/nef/decrypt.rs`,
  `rawler/src/decompressors/packed.rs`, and
  `rawler/src/decompressors/ljpeg/huffman.rs`.
  License: LGPL-2.1. Used as a behavioral reference only.
- LibRaw at `cf02d66eb5c19d4e46df7aee0582e47a4918509a`, especially
  `src/decoders/decoders_dcraw.cpp`,
  `src/decoders/decoders_libraw.cpp`, `src/metadata/tiff.cpp`,
  `src/metadata/identify.cpp`, and `src/metadata/nikon.cpp`.
  License: LGPL-2.1 or CDDL-1.0. Used as a behavioral reference only.
- TIFF 6.0 and TIFF/EP for IFD, EXIF-pointer, strip, CFA, and integer storage
  behavior.

The production implementation is independently written. Rawler and LibRaw are
not production dependencies.

## Container behavior

- The raw image is the largest non-reduced CFA IFD with strip storage and one
  sample per pixel.
- Nikon type-2 maker notes begin with `Nikon\0\x02`. The embedded byte-order
  marker is at bytes 10-11, and the first maker IFD starts at byte 18.
  Maker-note value offsets are based at byte 10 of the maker payload.
- TIFF and maker-note byte order must agree for this slice. A mismatch,
  headerless maker note, non-CFA image, multi-sample image, tile storage, or
  reduced-linear image is unsupported.
- Compression mode is read from maker tag `0x93`. Newer Z cameras may instead
  store the mode as a little-endian `u16` at offset 10 of tag `0x51`.

## Sample layouts

- Mode 2 is the legacy uncompressed family. Physical packing is selected from
  the validated byte count and row geometry.
- Mode 5 reads 32-bit little-endian words and extracts 12-bit samples MSB-first
  from the assembled word stream. For width 8, bytes
  `21 43 65 87 ba dc fe 10 32 54 76 98` decode to
  `0876 0543 0211 00fe 0dcb 0a98 0765 0432`.
- Mode 6 stores tightly packed 12-bit little-endian CFA rows.
- Mode 7 stores 12-bit samples in 16-bit words.
- Mode 9 stores packed 12-bit little-endian rows. The row stride is derived
  from validated strip byte counts rather than from Rawler's general
  uncompressed-data padding heuristic.
- Mode 10 stores either 14-bit samples in 16-bit words or packed 14-bit
  little-endian rows, selected by byte count.
- TIFF compression `1` with 12- or 14-bit samples stores unpacked 16-bit
  words. Packed mode 14 is not valid in big-endian byte order.
- Output allocations occur only after dimensions, strip sizes, row strides,
  and checked sample counts have been validated.

## Metadata

- Nikon `CropArea` (`0x45`) stores `[left, top, width, height]`. It maps to
  DNG `ActiveArea` as `[top, left, top + height, left + width]`.
- Black level (`0x3d`) has four values in CFA-position order. Values are
  relative to a 14-bit scale, so 12-bit samples use a two-bit right shift.
- White level is derived as `(1 << BitsPerSample) - 1` in this slice. Some
  camera profiles have a lower saturation point, which is deferred to camera
  profiles.
- White balance precedence is tag `0x0c`, then tag `0x97`. WB0 stores four
  rational gains in `[red, green1, green2, blue]` order.
- WB1 tag `0x97` starts with a four-character BCD version. This slice supports
  versions `0100`, `0102`, `0103`, `0204`, `0205`, and `0206`.
- WB1 versions `0100`, `0102`, and `0103` use big-endian `u16` gains at the
  offsets implemented by Rawler. Encrypted versions `0204`, `0205`, and
  `0206` decrypt a 280-byte block using the camera serial (`0x1d`) and Nikon
  key (`0xa7`); shutter count is not a WB key input.
- Both green channels are averaged. The DNG `AsShotNeutral` is
  `[green / red, 1, green / blue]`.
- Missing serial or key data leaves `AsShotNeutral` absent. No camera fallback
  table is synthesized.
- `CFAPattern` is interpreted in full-frame raw coordinates.
  `ProcessMetadata::bayer_pattern` continues to use the existing DNG
  conversion behavior, including its active-area phase handling.
- Nikon black-level values are tiled from the full-frame raw origin. A
  12-bit layout scales the four maker-note values down by two bits.
- `ProcessMetadata` records white-balance and black-level provenance as typed
  `MetadataSource` values. Provenance is diagnostic API only and is not part
  of the golden process hash or tracker CSV schema.

## Corpus survey

The pointer-only corpus snapshot was inspected by fetching at most the first
1 MiB of each `.NEF` path and parsing TIFF and maker-note metadata only. RAW
payloads were not materialized in the worktree.

Accepted physical layouts found in the corpus:

| Mode | Bits | Rows | Coverage |
| ---: | ---: | ---: | --- |
| 2 | 12 | 1 | `Nikon/D100/DSC_2376.NEF` |
| 5 | 12 | 0 | synthetic-only |
| 6 | 12 | 10 | real |
| 7 | 12 | 0 | synthetic-only |
| 9 | 12 | 0 | synthetic-only |
| 10 | 14 | 6 | real |

Real mode 6 examples include:

- `Nikon/D500/Nikon_D500_RAWL_12bit_Uncompressed.NEF`
- `Nikon/D810/DSC_1887.NEF`
- `Nikon/D850/Nikon-D850-12bit-uncompressed.NEF`
- `Nikon/Z 6/DSC_0753.NEF`
- `Nikon/Z 7/4-Nikon-Z7-RAW-12bit-uncompressed-L.NEF`

Real mode 10 examples include:

- `Nikon/D6/D06_0312.NEF`
- `Nikon/Z 6/DSC_0750.NEF`
- `Nikon/Z 7/1-Nikon-Z7-RAW-14bit-uncompressed-L.NEF`

The requested WB1 versions occur in the corpus:

| WB1 version | Files | Example |
| --- | ---: | --- |
| `0100` | 1 | `Nikon/D100/DSC_2376.NEF` |
| `0102` | 2 | `Nikon/D2H/D2A_0005.NEF` |
| `0103` | 2 | `Nikon/D70/20170902_0047.NEF` |
| `0204` | 4 | `NIKON CORPORATION/NIKON D2X/_ABA4604.NEF` |
| `0205` | 1 | `Nikon/D50/DSC_5155.NEF` |
| `0206` | 1 | `Nikon/D2Hs/DSC_5858.NEF` |

Four headerless older maker notes and one truncated survey prefix are outside
this slice. They remain unsupported or require fixture investigation.

## Known limits

- Mode 3 uses Nikon Huffman streams and is implemented at 12 and 14 bits.
  Lossy modes 1 and 4 remain unsupported by policy.
- Mode 8 small raw, reduced-linear output, HE/HE* modes 13/14, multi-sample
  files, and non-Bayer layouts are unsupported.
- Coolpix interlacing, camera-specific width overrides, and older headerless
  maker notes are deferred.
- The mode-5, mode-7, and mode-9 paths have synthetic coverage only in this
  corpus snapshot.

## Lossless Huffman verification

The implementation uses the pinned Rawler and LibRaw revisions above as
behavioral references. The owner approved on 2026-11-11 transcribing only the
numeric counts and symbols for the two lossless trees. No reference code,
comments, or naming is copied, and lossy or split trees are deliberately not
transcribed.

### Metadata and curve

- Rawler prefers maker tag `0x0096` (`NefMeta2`) and falls back to `0x008c`
  (`NefMeta1`). LibRaw records the metadata offset from either tag. Every
  mode-3 file in the pinned corpus uses `0x0096`.
- Both references read two version bytes, optionally skip 2110 bytes when the
  first byte is `0x49` or the second is `0x58`, then read four initial
  predictors as 16-bit maker-order values, followed by a 16-bit curve-point
  count.
- Every mode-3 corpus file has version bytes `46 30` and four predictors.
  Rawler selects the lossless tree when the first version byte is `0x46`.
  LibRaw's curve parser likewise leaves the identity curve in place for
  version `0x46`.
- For version `0x4630`, the curve-point count field is present but the
  referenced decoders do not consume curve entries. The effective
  linearization is identity. Its WhiteLevel is therefore
  `(1 << BitsPerSample) - 1`: 4095 for 12-bit files and 16383 for 14-bit
  files. No lower camera-specific saturation point is inferred.
- Rawler applies its dithering lookup even to the identity curve. That lookup
  emits either the exact predictor or one code below it. Fornax keeps the
  lossless predictor exact; real-file comparisons therefore require Rawler's
  reference value to equal the Fornax value or the value immediately below it.
  The selected D850 12-bit and Z8 14-bit fixtures satisfy that relationship
  sample-for-sample while preserving Fornax's exact lossless samples.
- Versions whose metadata does not select the lossless identity path remain
  unsupported for mode 3. They are not reinterpreted through a lossy tree.

### Huffman trees

The two-source agreement is exact. Rawler table index 2 and the first
12-bit lossless table selected by LibRaw contain the same counts and symbols;
Rawler index 5 and LibRaw's corresponding 14-bit table likewise agree.

| Bits | Rawler index | Counts by code length 1..16 | Symbols |
| ---: | ---: | --- | --- |
| 12 | 2 | `0,1,4,2,3,1,2,0,0,0,0,0,0,0,0,0` | `5,4,6,3,7,2,8,1,9,0,10,11,12` |
| 14 | 5 | `0,1,4,2,2,3,1,2,0,0,0,0,0,0,0,0` | `7,6,8,5,9,4,10,3,11,12,2,0,1,13,14` |

Each table is a complete canonical prefix code. The Rust representation is
format data, and `crates/fornax-codec` has a completeness test that constructs
both tables and requires the canonical code space to end exactly at the
16-bit boundary.

For each symbol `len`, read `len` extra bits. With `value` equal to those bits,
the difference is `value` when the sign bit is set, otherwise
`value - ((1 << len) - 1)`. A zero-length symbol contributes zero. The
lossless trees have no additional shift component.

Each row first decodes one difference for each CFA column phase and adds it to
the corresponding initial vertical predictor for the row parity. Later samples
in the row first add their difference to the preceding sample of the same
phase. Rows are processed serially because the next row of that parity depends
on vertically accumulated predictors.

### Lossy and strip conclusions

- Modes 1 and 4 use the same predictor and difference structure, but metadata
  can select lossy trees and a row at which an after-split tree takes over.
  They are deferred by policy; no lossy or split table is included.
- All 88 mode-3 files in the pinned corpus contain one strip. Supporting
  multiple independently coded mode-3 strips is outside this slice, so a
  multi-strip mode-3 layout is `Unsupported`.
- A one-strip mode-3 IFD may declare `RowsPerStrip` greater than the image
  height. The D7100 samples use 4022 declared rows for a 4020-row image, so
  validation accepts any full-width single strip whose declared row capacity
  covers the image and decodes exactly the image height.
- Mode 3 tree selection is determined by TIFF `BitsPerSample`: 12 selects the
  index-2 table and 14 selects the index-5 table.

### Corpus survey

The metadata-only survey read at most the first 2 MiB of each NEF through the
LFS batch endpoint and did not materialize RAW payloads. Of 277 NEF paths, 273
had parseable TIFF/type-2 maker metadata; four legacy D1/D1H/D1X files have
non-type-2 maker notes and remain unsupported. The 88 mode-3 paths all use
maker tag `0x0096`, metadata version `0x4630`, and one strip.

| Bits | Cameras (file count) | Files |
| ---: | --- | ---: |
| 12 | D3 (1), D300 (1), D300S (1), D3S (1), D4 (1), D4S (1), D500 (3), D6 (3), D600 (1), D610 (1), D700 (1), D7000 (1), D7100 (1), D7200 (1), D750 (1), D7500 (1), D780 (1), D800 (1), D810 (1), D850 (3), Df (1), Z fc (2), Z 30 (2), Z 5 (1), Z 50 (1), Z 6 (3), Z 6_2 (1), Z 7 (3), Z 7_2 (1) | 41 |
| 14 | D3 (1), D300 (1), D300S (1), D3S (1), D3X (1), D4 (1), D4S (1), D500 (1), D5100 (1), D6 (1), D600 (1), D610 (1), D700 (1), D7000 (1), D7100 (1), D7200 (1), D750 (1), D7500 (1), D780 (1), D800 (1), D800E (1), D810 (1), D850 (1), Df (1), Z fc (2), Z50_2 (3), Z5_2 (3), Z6_3 (4), Z 30 (2), Z 5 (1), Z 50 (1), Z 6 (1), Z 6_2 (1), Z 7 (1), Z 7_2 (1), Z 8 (1), Z 9 (1), Z f (1) | 47 |

Both bit depths and multiple Z-series bodies have real mode-3 coverage.
