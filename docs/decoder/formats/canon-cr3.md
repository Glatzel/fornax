# Canon CR3 Format

This format guide defines the supported container, compression, metadata, and completion requirements for Canon CR3.

**Tier:** 1

**Status:** Implemented for the audited lossless CRX slice. C-RAW and
ambiguous CMP1 shapes are typed unsupported.

## Container structure

CR3 and CRM are ISO Base Media File Format/QuickTime-style BMFF files. The
reader parses box sizes and types, `ftyp` compatibility (`crx`), `moov`,
`trak`, media/sample tables, `CRAW` sample entries, `mdat`, CTMD metadata, CMT
metadata IFDs, and the Canon-specific CMP1 and IAD1 boxes.

The raw CRAW track and sample are selected explicitly. The sample byte range
comes from `stco`/`co64`, `stsz`, and related tables; the image dimensions and
CFA come from CMP1/IAD1. Preview tracks must not be selected as raw data.

## Compression schemes

Implemented scope:

- Canon CRX lossless compression with four planes and zero transform levels;
- CR3 and CRM containers with the `crx` compatible brand.

Deferred:

- Canon CRX lossy/C-RAW compression;
- four-plane colour-difference reconstruction;
- preview, HEIF, and auxiliary dual-pixel decoding.

The CRX codec is wavelet-based with JPEG-LS-inspired prediction,
adaptive Golomb-Rice coding, run-length coding, and inverse quantization/
reconstruction for lossy bands. Multi-plane CMP1 data is reduced to the
single-plane mosaic expected by Fornax. This slice validates only the lossless
zero-level path. The header retains the encoding type and transform levels so
future lossy work can be added without changing the container API.

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
- Lossy reconstruction is deferred. If added, deterministic decoding must
  match reference digests exactly rather than use a visual tolerance.
- HEIF/JPEG previews are not raw payloads.

## Validated lossless discriminator

The accepted tuple is CMP1 version `0x0100`, four planes, encoding type zero,
zero wavelet levels, MDAT header size `0x70`, CFA layout `0..=3`, and 8-16 bit
declared precision. The targeted R5 and R6 Mark II C-RAW files use version
`0x0200` with three levels. Every unobserved or ambiguous tuple returns
`Unsupported`.

The full 110-file audit records 28 lossless rows, 51 lossy rows with three
wavelet levels, and 31 zero-level rows whose remaining CRX layout is ambiguous.
The latter two groups remain typed unsupported and are listed in the reviewed
tracker baseline.

## References

- [Canon CR3 documentation](https://github.com/lclevy/canon_cr3), plus ISO
  BMFF box definitions.
- `external/dnglab` at `62b77de1a206d467e910d55199191068b45a9cc1`:
  `rawler/src/decoders/cr3.rs`, `rawler/src/formats/bmff/`, and
  `rawler/src/decompressors/crx/`.
- `external/libraw` at `cf02d66eb5c19d4e46df7aee0582e47a4918509a`:
  CR3/CRX reference behavior.

CRX is implemented from public descriptions, the US 2016/0323602 A1 wavelet
description, T.87/JPEG-LS run and prediction rules, and observed bitstreams.
Rawler and LibRaw remain LGPL development references and are not production
dependencies.

## Completion gates

- Targeted R5 and R6 Mark II lossless mosaics match Rawler reference digests
  exactly.
- Targeted C-RAW files return typed unsupported-lossy errors.
- The selected CRAW track, sample index, CMP1 geometry, IAD1 crop, black level,
  white level, and WB are covered by targeted and synthetic tests.
- A standard 2x2 CR3 succeeds through the U16, cropped `f32`, and DNG writer
  handoff; non-2x2 layouts are rejected before preprocessing.
- The remote 110-file workflow classifies pass, unsupported-lossy, and other
  typed unsupported rows without a local full-corpus run.
