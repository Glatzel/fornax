# Canon CR3 format plan

**Tier:** 1

## Container structure

CR3 and CRM are ISO Base Media File Format/QuickTime-style BMFF files. The
reader parses box sizes and types, `ftyp` compatibility (`crx`), `moov`,
`trak`, media/sample tables, `CRAW` sample entries, `mdat`, CTMD metadata, CMT
metadata IFDs, and the Canon-specific CMP1 and IAD1 boxes.

The raw CRAW track and sample are selected explicitly. The sample byte range
comes from `stco`/`co64`, `stsz`, and related tables; the image dimensions and
CFA come from CMP1/IAD1. Preview tracks must not be selected as raw data.

## Compression schemes

Initial scope:

- Canon CRX lossless compression;
- Canon CRX lossy/C-RAW compression;
- uncompressed or lightly transformed variants observed in accepted samples.

The CRX codec is wavelet-based with JPEG-LS-inspired prediction,
adaptive Golomb-Rice coding, run-length coding, and inverse quantization/
reconstruction for lossy bands. Multi-plane CMP1 data is reduced to the
single-plane mosaic expected by Fornax.

## Metadata to DNG tags

Map or preserve:

- `Make`, `Model`, `CameraSerialNumber`, and unique image ID;
- `ActiveArea` from IAD1 active-area offsets;
- `DefaultCropOrigin` and `DefaultCropSize` from IAD1 crop offsets;
- `CFARepeatPatternDim`, `CFAPattern`, `CFAPlaneColor`, and `CFALayout`;
- `BlackLevelRepeatDim`, `BlackLevel`, and `WhiteLevel`;
- `AsShotNeutral` or camera white-balance coefficients;
- `Orientation`, EXIF identity, lens, and exposure tags;
- XMP packet and safe maker-note metadata.

IAD1 offset conventions often use inclusive right/bottom coordinates; convert
to Fornax half-open rectangles once during parsing.

## Known variants and pitfalls

- RAW versus C-RAW versus CRM movie samples use different intent even when the
  CRX bitstream is similar.
- Some C-RAW/light-raw variants already have white balance or other
  processing applied; do not reapply camera corrections.
- IAD1 crop and active-area endpoints can exceed CMP1 dimensions by a few
  pixels on known firmware and crop modes.
- The raw track can contain multiple samples; the selected sample index must
  be explicit.
- CRX uses four planes in the compressed representation but not a four-color
  CFA.
- Dual-pixel files may carry an auxiliary image; the primary mosaic is the
  decode target.
- Lossy reconstruction must preserve the declared output geometry and not
  silently claim lossless pixel equality with a lossless reference.
- HEIF/JPEG previews are not raw payloads.

## References to consult

- [Canon CR3 documentation](https://github.com/lclevy/canon_cr3), plus ISO
  BMFF box definitions.
- `external/dnglab` at `62b77de1a206d467e910d55199191068b45a9cc1`:
  `rawler/src/decoders/cr3.rs`, `rawler/src/formats/bmff/`, and
  `rawler/src/decompressors/crx/`.
- `external/libraw` at `cf02d66eb5c19d4e46df7aee0582e47a4918509a`:
  CR3/CRX reference behavior.

CRX is the highest algorithm-risk tier-1 codec. Read the licence constraints
in the architecture before deriving code; implement from public descriptions
and observed bitstreams.

## Definition of done

- Accepted CR3 corpus cameras report `pass=1`, or an explicitly reviewed
  `unsupported_camera`/`unsupported_variant` row for a known camera.
- Lossless CRX matches the reference sample codes exactly after geometry and
  CFA alignment.
- Lossy/C-RAW has a documented comparison rule that does not claim exact
  equality when the reference decoding path is also lossy.
- The selected CRAW track, sample index, CMP1 geometry, IAD1 crop, black level,
  white level, and WB are covered by synthetic or tiny metadata tests.
- A standard 2x2 CR3 succeeds at the handoff; a non-2x2 CR3 would be rejected
  with `HandoffError::UnsupportedCfa`, never passed to the preprocessor.
