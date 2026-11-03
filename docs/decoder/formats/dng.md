# DNG format plan

**Tier:** 0, first implementation

**Specification:** Adobe Digital Negative Specification 1.7.1.0, September
2023, linked from Adobe's Digital Negative page. DNG extends TIFF 6.0/TIFF-EP,
and DNG 1.7 includes BigTIFF.

## Scope

In scope:

- reading every still-image compression and storage combination that appears
  in the accepted DNG corpus;
- preserving all DNG raw metadata needed by Fornax and round-tripping unknown
  well-formed tags where safe;
- exporting uncompressed DNG first and lossless JPEG-92 DNG second;
- arbitrary CFA layouts through the native model and DNG writer;
- conventional 2x2 RGB Bayer through the current preprocessor handoff.

Out of scope initially:

- preview, thumbnail, mask, depth, and video image stages as raw sources;
- lossy writing;
- CinemaDNG-specific semantics;
- vendor private metadata interpretation beyond preserving it.

## Container structure

The reader supports classic TIFF and BigTIFF headers, both byte orders, IFD
chains, SubIFDs, and typed tag values. It selects the raw image IFD by
`NewSubFileType`, `PhotometricInterpretation` (`CFA` or `LinearRaw`), geometry,
and compression, rather than taking the first IFD.

Raw samples may be stored in strips or tiles. DNG layout tags must be applied
in specification order:

- `SubTileBlockSize`;
- `RowInterleaveFactor`, and DNG 1.7.1 `ColumnInterleaveFactor`;
- optional `LinearizationTable`;
- `OpcodeList1`, `OpcodeList2`, and `OpcodeList3`.

Offsets are validated after every read. Total sample counts, tile counts, strip
counts, and allocation lengths use checked arithmetic.

## Compression schemes

Reader targets:

| TIFF compression | Storage | Required behavior |
| --- | --- | --- |
| Uncompressed (`1`) | Strips/tiles | Packed unsigned and IEEE samples. |
| Lossless JPEG-92 (`7`) | Strips/tiles | DNG JPEG tables, predictor, point transforms, and the pre-1.1 16-bit deviation. |
| Deflate (`8`) | Strips/tiles | Supported predictor rules for integer and floating-point data. |
| JPEG XL (`52546`) | Strips/tiles | DNG 1.7.1 integer CFA reads. |
| Lossy JPEG (`34892`) | Strips/tiles | Read only if accepted corpus files contain a raw CFA image; reject with a typed unsupported-variant error otherwise. |

Writer targets:

1. Uncompressed strips with exact `MosaicData` samples.
2. Lossless JPEG-92 integer strips after the uncompressed path is proven.

The writer does not silently convert float samples to integer or change
`WhiteLevel` to make compression possible.

## Metadata to DNG tags

Native DNG values are copied into `DngMetadata` with their original TIFF type.
At minimum, preserve:

- identity: `Make`, `Model`, `UniqueCameraModel`, `CameraSerialNumber`;
- geometry: `ActiveArea`, `DefaultCropOrigin`, `DefaultCropSize`,
  `MaskedAreas`, `Orientation`;
- CFA: `CFARepeatPatternDim`, `CFAPattern`, `CFAPlaneColor`, `CFALayout`;
- levels: `BlackLevelRepeatDim`, `BlackLevel`, `BlackLevelDeltaH`,
  `BlackLevelDeltaV`, `WhiteLevel`, `LinearizationTable`;
- white balance and calibration: `AsShotNeutral`, `AsShotWhiteXY`,
  `AnalogBalance`, `CameraCalibration1/2`, `ColorMatrix1/2`,
  `ForwardMatrix1/2`, `CalibrationIlluminant1/2`;
- rendering hints: `BaselineExposure`, `BaselineNoise`, `BaselineSharpness`,
  `BayerGreenSplit`, `DefaultScale`;
- image transforms: `OpcodeList1/2/3`.

Unknown tags are retained in the typed map. On write, tags are either copied
verbatim, regenerated with new offsets, or dropped with a typed writer warning
if their value depends on a changed IFD layout.

## Known variants and pitfalls

- `DNGVersion` and `DNGBackwardVersion` are different compatibility
  declarations.
- BigTIFF uses 8-byte offsets and the `LONG8`, `SLONG8`, and `IFD8` field
  types.
- A file may contain many IFDs; only a CFA or supported linear raw IFD is the
  decode source.
- `CFAPattern` is relative to `ActiveArea`, not necessarily to the full stored
  frame.
- `CFALayout` must be retained even when the CFA repeat is rectangular.
- Black level may be scalar, repeating, per-row, or per-column; all forms must
  combine according to the DNG rules.
- Applying a linearization table or opcode list changes the pixel stage and
  therefore changes which tags may be preserved.
- DNG opcodes and tile boundaries have historically exposed malformed offsets
  and extreme counts in fuzz inputs.
- Lossless JPEG has different predictor/point-transform conventions from
  ordinary JPEG and a legacy 16-bit bug before DNG 1.1.
- JPEG XL compression parameters were refined in 1.7.1.0; do not infer them
  from generic JPEG XL defaults.
- Linear raw images and RGB previews must not be reported as CFA mosaics.
- A standard 2x2 CFA is the only DNG layout that may enter the current
  preprocessor.

## References to consult

- [Adobe Digital Negative page](https://helpx.adobe.com/camera-raw/digital-negative.html),
  DNG Specification 1.7.1.0 (September 2023), TIFF 6.0 notes.
- [DNG 1.7.1.0 PDF](https://download.adobe.com/pub/adobe/dng/DNG_Spec_1_7_1_0.pdf).
- `external/dnglab` at `62b77de1a206d467e910d55199191068b45a9cc1`:
  `rawler/src/decoders/dng.rs`, `rawler/src/dng/writer.rs`,
  `rawler/src/formats/tiff/`, `rawler/src/decompressors/deflate.rs`,
  `rawler/src/decompressors/ljpeg/`, and DNG opcode modules.
- `external/libraw` at `cf02d66eb5c19d4e46df7aee0582e47a4918509a`:
  DNG parsing and DNG SDK-derived interpretation paths.

The references are LGPL-2.1 (rawler; LibRaw also offers CDDL-1.0). Use them to
understand observed behavior, not as copied production code.

## Definition of done

- Synthetic DNG tests cover classic TIFF and BigTIFF, both byte orders,
  strips/tiles, integer and float samples, and every supported reader
  compression.
- A small synthetic CFA DNG exports and reads back with exact samples and
  required tags, for both uncompressed and lossless JPEG-92 output.
- Corpus rows for accepted DNG cameras have `pass=1` under the corpus
  definition, with exact stored-sample agreement where Rawler or LibRaw can
  provide a comparable image.
- At least one X-Trans-shaped DNG decodes and exports while a 2x2 Bayer DNG
  reaches the preprocessor handoff; every other layout returns
  `HandoffError::UnsupportedCfa`.
- The production dependency tree contains no C/C++ or `-sys` crate.
