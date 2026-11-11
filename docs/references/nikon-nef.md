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

- Modes 1, 3, and 4 use Nikon Huffman streams and are not decoded here.
- Mode 8 small raw, reduced-linear output, HE/HE* modes 13/14, multi-sample
  files, and non-Bayer layouts are unsupported.
- Coolpix interlacing, camera-specific width overrides, and older headerless
  maker notes are deferred.
- The mode-5, mode-7, and mode-9 paths have synthetic coverage only in this
  corpus snapshot.
