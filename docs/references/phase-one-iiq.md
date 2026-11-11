# Phase One/Leaf IIQ references

## Sources

- Rawler `0.8.0`, LGPL-2.1: `src/decoders/iiq.rs`,
  `src/imgop/spline.rs`, `src/pumps.rs`, and generated camera metadata.
- dnglab commit `62b77de1a206d467e910d55199191068b45a9cc1`, LGPL-2.1:
  Phase One decoder and camera metadata.
- LibRaw commit `cf02d66eb5c19d4e46df7aee0582e47a4918509a`, LGPL-2.1 or
  CDDL-1.0: `src/metadata/mediumformat.cpp`,
  `src/decoders/load_mfbacks.cpp`, `src/decoders/decoders_libraw.cpp`, and
  `src/utils/phaseone_processing.cpp`.

## Container behavior

- Classic little-endian TIFF header; first IFD and preview resources can sit
  near EOF.
- Phase One private header at file offset 8 contains `Raw`, a private IFD
  offset, and 16-byte maker entries.
- Payload, row-offset, string, white-balance, split-black, and sensor-correction
  offsets are relative to the private header base.
- Maker dimension tags describe the stored raster; margin and image-width tags
  define the active image.

## Sample behavior

- IIQ-L and IIQ-L16 use a per-row little-endian 32-bit word stream consumed
  MSB-first. Length selectors update two alternating predictors at each
  eight-pixel boundary. The final `width % 8` samples are direct 16-bit values.
- IIQ-S shares IIQ-L framing. Values below 256 pass through the fixed
  `round(i*i / 3.969)` curve and are shifted from 14 to 16 bits; larger values
  remain unshifted.
- IIQ-Sv2 uses an initial three-bit base width and eight-pixel clusters with
  even/odd bit indicators. Decoded aggregates outside 14 bits trigger the
  bounded recovery path.
- Uncompressed samples are little-endian 14-bit codes scaled by four.

## Correction behavior

- Correction order in the target Rawler behavior is black, polynomial curves,
  quadrant multipliers, combined/linearization curves, bad columns, then flat
  fields.
- Split black levels use signed row/column offsets selected by split row and
  column coordinates.
- Defect types 131 and 137 repair complete bad columns. Single-pixel type 129
  entries are left for later processing.
- Flat-field CFA lookup subtracts the sensor top/left margins before selecting
  the 2x2 color plane.
- The Rawler spline used for quadrant curves reduces to piecewise linear
  interpolation in the target behavior.
- The P65+ corpus uses a short quadrant-linearization payload that is not the
  1028-byte table consumed by the supported correction path; Fornax leaves
  this variant typed unsupported.
- P45+ files can expose only one split-black table. The reference path ignores
  incomplete split metadata, and Fornax follows that behavior.

## Metadata mapping

- `AsShotNeutral` is `[green/red, 1, green/blue]` from the three source gains.
- Corrected samples export black level zero and white level 65535.
- Canonical DNG metadata carries source Make/Model, orientation, active area,
  default crop, and explicit 2x2 CFA profile data.
- No camera color matrix is synthesized from the LGPL camera database.

## Implementation boundary

Fornax's codec and container implementation is independently written from the
behavioral observations above. No Rawler or LibRaw implementation source,
comments, or naming is copied into production crates. Rawler is used only as a
development-time fixture oracle in tests.

Modes 1 and 2, Leaf `.MOS`, Phase One `.TIF`, preview extraction, unknown
camera profiles, and non-2x2 pipeline input are outside the implemented tier.
