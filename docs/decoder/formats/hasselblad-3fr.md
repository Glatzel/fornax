# Hasselblad 3FR format support

**Tier:** 2, implemented for the accepted conventional-Bayer corpus.

## Container structure

3FR files are classic little-endian TIFF containers. The root IFD contains
previews, identity, color matrices, white balance, and optional Exif. The
full-resolution raw image is a CFA SubIFD containing `WhiteLevel`, strip
geometry, and optional black level or crop tags.

The decoder selects the largest non-reduced CFA IFD with `WhiteLevel` and strip
storage. Accepted raw IFDs have one 16-bit integer sample per pixel and no CFA
pattern tags; those values come from explicit camera profiles.

## Compression schemes

Two payload layouts are accepted:

- TIFF compression 1 stores exactly `width * height` little-endian 16-bit
  words. `StripByteCounts` must match that word count.
- TIFF compression 7 contains a single-component 16-bit SOF3 stream using
  predictor 8. The decoder reads two Huffman lengths, then their two attached
  differences, for each pixel pair. Predictors reset to `0x8000` at every row.
  The bitstream uses Rawler's 3FR `MSB32` word order and consumes a 16-bit
  payload for difference class 16.

Compression-7 strip byte counts are not used as payload bounds because several
accepted files record values larger than the file. The checked strip offset
bounds the payload and entropy decoding continues through EOF.

Other compression identifiers, tiled storage, multi-sample pixels, non-16-bit
samples, restart intervals, point transforms, and non-predictor-8 streams
return typed `DecodeError::Unsupported` or `DecodeError::InvalidInput` errors.

## Camera profiles

The accepted tagless models use 2x2 RGGB profiles. Full-frame active areas are
used for:

- `Hasselblad H3D`
- `Hasselblad H4D`
- `Hasselblad CFV`
- `Hasselblad96`
- `Hasselblad ixpressCF132`
- `Hasselblad X2D 100C`
- `Hasselblad CFV 100C/Electronic Shutter`

`Hasselblad X1D` and `Hasselblad X1D II 50C` remove inactive borders
`[left 46, top 96, right 58, bottom 0]`. The CFV 50C II corpus file reports the
X1D II unique model and uses the same borders.

Source CFA tags take precedence when present. Well-formed non-2x2 CFA data can
be decoded and exported as CFA DNG, but `process_metadata` and
`cropped_mosaic` reject it before preprocessing.

## Metadata to DNG tags

The DNG metadata mapper preserves or synthesizes:

- `Make`, `Model`, `UniqueCameraModel`, `RawDataUniqueID`, `Orientation`;
- full dimensions, `ActiveArea`, and active-area-relative default crop;
- CFA repeat/pattern/plane colors/layout;
- `BlackLevel` and `BlackLevelRepeatDim`;
- `WhiteLevel`;
- `ColorMatrix1`, `CalibrationIlluminant1`, and `AsShotNeutral` when present;
- allowlisted Exif exposure, sensitivity, date, focal length, lens, and serial
  fields.

The generic `DngWriter` can export every decoded mosaic without byte-identical
container regeneration.

## Out of scope

FFF, Imacon/merged/multi-shot variants, unknown tagless camera models, unknown
CFA layouts, tiled data, and unsupported compression families remain outside
this implementation. The accepted manifest files remain limited to `.3FR`.

## Definition of done

- The 11 accepted 3FR corpus cameras are present in the committed manifest.
- All accepted uncompressed and predictor-8 samples match Rawler's low-level
  codec exactly and complete the conventional Bayer handoff.
- Compressed and uncompressed samples round-trip exactly through the generic
  DNG writer.
- The decoder-tracker baseline records all accepted 3FR rows as passing.

Behavioral provenance and licensing constraints are recorded in
[Hasselblad 3FR references](../../references/hasselblad-3fr.md).
