# Panasonic RW2 Format

This format guide defines the supported container, compression, metadata, and completion requirements for Panasonic RW2.

**Tier:** 2

## Scope

`fornax-raw-panasonic` natively decodes single-shot Panasonic RW2 files with conventional
2x2 RGB Bayer storage:

- Panasonic raw-format versions 4, 5, 6, and 8;
- 12- or 14-bit compressed families where the version permits them;
- stamped 12-bit unpacked and packed-control layouts;
- V8 identity and nonlinear gamma tables, Huffman-coded differences, and
  multi-strip placement;
- 12-bit DMC-G85, 14-bit DC-GH5S, 14-bit DC-S1, and 16-bit DC-GH6 fixtures.

The decoder returns `RawMosaic::U16`, conventional `ProcessMetadata`, and
DNG-compatible metadata. It implements both shared decoder traits.

V7 is deferred. A V7 file returns typed `Unsupported`; no V7 codec will be
added until a real single-shot V7 file is available to validate against.
Legacy `.RAW`, raw-format versions 1-3, multi-exposure, pixel-shift,
high-resolution composites, and non-Bayer layouts are outside scope.

## Container Layout

RW2 uses classic TIFF layout with magic `85` instead of `42`. The first
primary IFD containing `RawDataOffset` (`0x0118`) is selected. If none exists,
the first primary IFD containing `StripOffsets` (`0x0111`) is selected.
Offsets and count arithmetic are checked before slicing or allocation.

The raw-format version is `0x002d`. The Panasonic compression tag is `0x000b`.
The supported compressed files use compression `34316`; the V8 fixture omits
the compression tag and is accepted only for V8.

| Version | Compression | Selected codec |
| --- | --- | --- |
| 4 | `34316` | Panasonic V4; split blocks when `0x0118` is used |
| 5 | `34316` | Panasonic V5 |
| 6 | `34316` | Panasonic V6 |
| 7 | any | `Unsupported` pending a single-shot validation file |
| 8 | absent or `34316` | Panasonic V8 |
| other | any | `Unsupported` |

For a 12-bit `StripOffsets` layout, payload size distinguishes
`PanasonicUnpacked12` from `PanasonicPacked12Control`. Packed-control rows use
16-byte groups containing ten pixels and one control byte; final short groups
are decoded without inventing pixels.

Nonzero `0x0121` or a nonzero CameraIFD `0x1001` marks a multi-shot file and is
rejected before sample decoding. Some V8 high-resolution composites have no
marker. Layouts above 64,000,000 pixels are rejected as unsupported
single-shot candidates; normal Panasonic single-shot sensors remain below this
bound. The tested DC-S1R2 multi-exposure split is additionally rejected by its
pinned V8 strip placement.

## Metadata Mapping

Sensor border tags `0x0004`-`0x0007` are top, left, bottom, and right
coordinates. Right and bottom are exclusive. The full TIFF dimensions come
from `0x0002` and `0x0003`.

CFA tag `0x0009` maps `1`-`4` to RGGB, GRBG, GBRG, and BGGR at full-sensor
origin. DNG describes CFA relative to `ActiveArea`, so the pattern is shifted
by the active origin modulo two.

Default crop tags `0x002f`-`0x0032` are top, left, bottom, and right
coordinates, not width/height margins. DNG `DefaultCropOrigin` is relative to
`ActiveArea`; DNG crop size uses the exclusive lower/right coordinates. Older
files may write all four crop tags as zero placeholders; those mean the full
active area.

Metadata sources:

| Output | Source |
| --- | --- |
| Black level | `0x001c`-`0x001e`, plus 15 for V4 or absent `RawDataOffset` |
| White level | Maximum present nonzero `0x000e`-`0x0010`; otherwise `(1 << bits) - 1` |
| As-shot neutral | `0x0011`/`0x0012`, otherwise `0x0024`-`0x0026`, green normalized to one |
| Orientation | Raw-IFD TIFF tag `0x0112` |
| Serial number | Exif `0xa431`, when present |
| Lens model | Exif `0xa434`, when present |

Black levels and CFA are emitted in active-area spatial order for the shared
preprocessor. Camera color matrices are not synthesized.

## Safety

- Dimensions, sample counts, row sizes, bit ranges, strip offsets, table sizes,
  and output ranges use checked arithmetic.
- Payload and strip bounds are validated before allocating output.
- V8 parameter and strip constructors validate table lengths, Huffman symbols,
  shifts, dimensions, overlap, and placement. Decode validates full image
  coverage again.
- Codec tests sweep every truncation prefix for representative streams and
  mutate representative bytes without a fuzz dependency.

## Validation

Synthetic tests cover every codec family, V8 identity and nonlinear gamma,
stamp classification, invalid geometry, malformed Huffman tables, overflow,
truncation, and mutation. Four corpus fixtures match Rawler on every storage
sample and are committed in the golden decode set. DNG write/read tests cover
samples, active area, crop, CFA, black/white levels, neutral, and orientation.

The manual corpus selection is `**/*.[rR][wW]2` with `run_duckdb_tests`
disabled. RAW payloads remain in raw.pixls.us storage and are never committed
or uploaded.
