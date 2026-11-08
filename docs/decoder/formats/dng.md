# DNG format plan

**Tier:** 0

**Specification:** Adobe Digital Negative Specification 1.7.1.0, September
2023. DNG extends TIFF 6.0/TIFF-EP; DNG 1.7 adds BigTIFF.

## Implementation status

The current MVP lives in `fornax-dng`, with `fornax-decoder` as its
`IDecoder` adapter, and supports:

- classic TIFF little- and big-endian headers;
- classic BigTIFF headers;
- root IFD chains and `SubIFDs`;
- full-resolution CFA IFD selection rather than first-IFD selection;
- typed root and raw-IFD metadata preservation;
- uncompressed packed-integer and floating-point strips/tiles;
- lossless JPEG-92 strips/tiles with predictors 1 through 7 and 2- to 16-bit
  precision for supported sample layouts;
- Deflate, lossy JPEG, and JPEG XL raw chunks;
- 2x2 row/column interleave, linearization, and `OpcodeList1` pixel transforms;
- parallel strip/tile decode with bounded per-band scratch storage;
- conventional 2x2 RGB Bayer metadata conversion and active-area handoff;
- uncompressed DNG generation with exact sample and selected-tag readback;
- tiled lossless JPEG-92 generation for U8/U16 and lossless Deflate generation
  for U32/F32.

The accepted `colorchart-5D2-6000K.dng` fixture decodes to the full
`5792x3804` mosaic, crops to a `5634x3753` active area, and shifts from GBRG
at the full-frame origin to RGGB relative to the active area. Its metadata
reports a `2x2` black-level pattern of 1024 and a white level of 15600.

The pointer-only `raw.pixls.us` submodule and `decoder-tracker` are used for
wider manual validation:

- Canon EOS 5D Mark III DNG compression variants;
- Nikon D80 DNG with `LinearizationTable`;
- Sigma fp 14-bit DNG with two-component lossless JPEG tiles;
- Ricoh GR III 14-bit DNG;
- Sony ILCE-1M2 and Nikon Z 7 II DNGs generated once from their native raws
  through the Rawler reference converter.

The Canon CFA, Nikon D80, Sigma fp, Ricoh GR III, Sony ILCE-1M2, and Nikon
Z 7 II decoded samples match Rawler exactly after applying each file's active
area. Native Sony/Nikon raws are not treated as default-decoder coverage.

Lossless export is regenerated rather than byte-identical. Byte-identical
export requires retaining the source container, which the current public
decoded-data model does not expose.

Remaining DNG work:

- pre-DNG-1.1 lossless-JPEG compatibility mode;
- lossless-JPEG restart intervals, Huffman selector `0x10`, and multi-component
  subsampled or half-height tile variants;
- multi-channel `LinearRaw` decoding outside the current mono `RawMosaic`
  contract;
- legacy TIFF JPEG XL compression identifier `50002`;
- `SubTileBlockSize` and non-2x2 interleave factors;
- geometric/frontend opcodes outside the raw pixel domain;
- `OpcodeList2` and `OpcodeList3` execution in their downstream stages;
- automatic BigTIFF output for regenerated files exceeding classic TIFF limits;
- preview, thumbnail, mask, depth, and nested Exif/GPS preservation;
- accepted DNG corpus rows, identity metadata, geometry, stage errors, and CFA
  labels remain stable against the reviewed CSV baseline.

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
| Uncompressed (`1`) | Strips/tiles | Packed 1-32-bit integer and 16/24/32-bit float samples |
| Lossless JPEG-92 (`7`) | Strips/tiles | 2-16-bit precision, predictors 1-7, single-component or 1x1 interleaved components |
| Deflate (`8`) | Strips/tiles | Integer and float payloads with standard horizontal/float predictors |
| JPEG XL (`52546`) | Strips/tiles | Single-channel 8-16-bit mosaics |
| Lossy JPEG (`34892`) | Strips/tiles | Single-component 8-bit mosaics |
| Zstandard (`50000`, experimental) | Strips/tiles | Integer and float payloads |

Compression `50002` is the legacy TIFF JPEG XL identifier. It is not yet
represented by `fornax_tiff::CompressionMethod`, while `52546` is the DNG 1.7
identifier mapped to the single-channel JPEG XL decoder.

The approved codec policy is mandatory availability: zstd and all future codecs
must be compiled unconditionally without per-codec features. The current zstd
feature gate is a pending implementation gap, so current builds still require
the feature for compression `50000`.

Malformed values and layouts outside the active Bayer contract return
`DecodeError::InvalidInput` or the existing CFA boundary error. Unknown
compression identifiers are invalid input rather than stale unsupported
placeholders.

## Metadata

The decoder retains typed values in a `fornax_tiff::Directory`, an ordered
`BTreeMap<Tag, Value>`. Standard TIFF/TIFF-EP fields use `TiffTag`; DNG-only
fields use `DngTag`. The pipeline consumes at least:

- `ActiveArea`, `DefaultCropOrigin`, and `DefaultCropSize`;
- `CFARepeatPatternDim`, `CFAPattern`, `CFAPlaneColor`, and `CFALayout`;
- `BlackLevelRepeatDim`, `BlackLevel`, `BlackLevelDeltaH`,
  `BlackLevelDeltaV`, and `WhiteLevel`;
- `AsShotNeutral`, `AsShotWhiteXY`, and calibration/render hints;
- `Make`, `Model`, `UniqueCameraModel`, and `CameraSerialNumber`.

Unknown well-formed tags remain in the metadata map. `OpcodeList1` and
`LinearizationTable` affect `raw_mosaic`; `OpcodeList2` and `OpcodeList3`
remain available as metadata for their downstream stages. When regenerating a
container, offsets and layout-dependent image tags are regenerated or omitted.

`ActiveArea` defaults to the full decoded frame when absent, as seen in some
mobile DNGs. `DefaultCropOrigin`/`DefaultCropSize` default to that active area
when absent. A missing `CFAPlaneColor` defaults to RGB plane IDs.

## CFA handoff

The active pipeline accepts only conventional RGB 2x2 Bayer layouts:
`RGGB`, `BGGR`, `GRBG`, and `GBRG`. `CFAPattern` is interpreted relative to
`ActiveArea`. `cropped_mosaic<T>` crops the full `RawMosaic` and returns
`CroppedMosaic<T>` with the active-area-relative Bayer pattern in
`ProcessMetadata<T>`.

Other repeat sizes, plane colors, layouts, or CFA patterns return a typed
unsupported error. The decoder does not claim X-Trans, four-color, monochrome,
or arbitrary-CFA pipeline support. Multi-channel `LinearRaw` DNG images also
remain outside the mono mosaic contract even when their compression is
recognized.

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
- Production `fornax-tiff`, `fornax-dng`, and `fornax-decoder` dependencies
  remain free of C/C++ and `-sys` crates.
