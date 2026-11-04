# DNG format plan

**Tier:** 0

**Specification:** Adobe Digital Negative Specification 1.7.1.0, September
2023. DNG extends TIFF 6.0/TIFF-EP; DNG 1.7 adds BigTIFF.

## Implementation status

The current MVP lives in `fornax-core` and supports:

- classic TIFF little- and big-endian headers;
- root IFD chains and `SubIFDs`;
- full-resolution CFA IFD selection rather than first-IFD selection;
- typed root and raw-IFD metadata preservation;
- uncompressed integer and floating-point strips/tiles;
- lossless JPEG-92 strips/tiles with predictors 1 through 7;
- conventional 2x2 RGB Bayer metadata conversion and active-area handoff;
- uncompressed DNG generation with exact sample and selected-tag readback.

The accepted `colorchart-5D2-6000K.dng` fixture decodes to the full
`5792x3804` mosaic, crops to a `5634x3753` active area, and shifts from GBRG
at the full-frame origin to RGGB relative to the active area. Its metadata
reports a `2x2` black-level pattern of 1024 and a white level of 15600.

Default lossless-JPEG export currently returns
`DngWriteError::Unsupported`. Byte-identical export requires retaining the
source container, which the current public decoded-data model does not expose.

Remaining DNG work:

- BigTIFF inputs;
- Deflate and JPEG XL compression;
- lossy JPEG raw CFA inputs;
- pre-DNG-1.1 lossless-JPEG compatibility mode;
- row/column interleave factors, sub-tile blocks, linearization tables, and
  opcode lists;
- general lossless-JPEG recompression;
- corpus coverage for accepted DNG cameras.

## Container handling

The reader validates classic TIFF offsets and typed values before reading
sample payloads. It traverses root IFDs and `SubIFDs`, then selects the largest
non-reduced CFA or LinearRaw image directory. Preview, thumbnail, mask, depth,
and Exif images are not raw decode sources.

Raw samples may use strips or tiles. Strip/tile counts, dimensions, offsets,
byte counts, and the final sample buffer are checked with overflow-safe
arithmetic. Partial final chunks are copied only within the declared image
geometry.

## Compression

| TIFF compression | Storage | Current behavior |
| --- | --- | --- |
| Uncompressed (`1`) | Strips/tiles | 8-, 16-, and 32-bit integer plus 32-bit float samples |
| Lossless JPEG-92 (`7`) | Strips/tiles | 16-bit CFA tiles, restart-free, predictors 1-7 |
| Deflate (`8`) | Strips/tiles | Unsupported |
| JPEG XL (`52546`) | Strips/tiles | Unsupported |
| Lossy JPEG (`34892`) | Strips/tiles | Unsupported |

Unsupported compression returns `DecodeError::Unsupported`; malformed values
return `DecodeError::InvalidInput`.

## Metadata

The decoder retains typed DNG values in
`BTreeMap<DngTag, DngValue>`. The pipeline consumes at least:

- `ActiveArea`, `DefaultCropOrigin`, and `DefaultCropSize`;
- `CFARepeatPatternDim`, `CFAPattern`, `CFAPlaneColor`, and `CFALayout`;
- `BlackLevelRepeatDim`, `BlackLevel`, `BlackLevelDeltaH`,
  `BlackLevelDeltaV`, and `WhiteLevel`;
- `AsShotNeutral`, `AsShotWhiteXY`, and calibration/render hints;
- `Make`, `Model`, `UniqueCameraModel`, and `CameraSerialNumber`.

Unknown well-formed tags remain in the metadata map. When regenerating an
uncompressed container, offsets and layout-dependent image tags are regenerated
or omitted; values that cannot be serialized return a typed writer error.

## CFA handoff

The active pipeline accepts only conventional RGB 2x2 Bayer layouts:
`RGGB`, `BGGR`, `GRBG`, and `GBRG`. `CFAPattern` is interpreted relative to
`ActiveArea`. The active-area origin shifts the full-frame Bayer phase before
the preprocessor receives `DecodedRaw<T>`.

Other repeat sizes, plane colors, layouts, or CFA patterns return a typed
unsupported error. The decoder does not claim X-Trans, four-color, monochrome,
or arbitrary-CFA pipeline support.

## References

- [Adobe Digital Negative page](https://helpx.adobe.com/camera-raw/digital-negative.html)
- [DNG 1.7.1.0 specification](https://download.adobe.com/pub/adobe/dng/DNG_Spec_1_7_1_0.pdf)
- `external/dnglab` at
  `62b77de1a206d467e910d55199191068b45a9cc1`, specifically its DNG, TIFF,
  Deflate, lossless-JPEG, opcode, and writer modules.
- `external/libraw` at
  `cf02d66eb5c19d4e46df7aee0582e47a4918509a`, for observed DNG parsing and
  metadata behavior.

These references are LGPL-2.1 (Rawler; LibRaw also offers CDDL-1.0) and are
read for behavior only. Production code is implemented independently from the
specifications.

Implementation evidence and known lossless-JPEG differences are recorded in
[`docs/references/dng-lossless-jpeg.md`](../../references/dng-lossless-jpeg.md).

## Definition of done for future DNG expansion

- Synthetic tests cover BigTIFF, both byte orders, strips/tiles, integer and
  float samples, and every newly supported compression.
- A synthetic CFA DNG exports and reads back with exact samples and required
  tags.
- Accepted DNG corpus rows pass and preserve stored-sample agreement where a
  reference implementation can provide a comparable image.
- Unsupported CFA layouts return typed errors without leaking into the active
  preprocessor handoff.
- Production `fornax-core` dependencies remain free of C/C++ and `-sys`
  crates.
