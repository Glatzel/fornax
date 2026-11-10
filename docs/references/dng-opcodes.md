# DNG opcode reference notes

## Authoritative sources

- [Adobe DNG Specification 1.7.1.0](https://download.adobe.com/pub/adobe/dng/DNG_Spec_1_7_1_0.pdf),
  September 2023, chapter 7. This defines opcode-list framing, regions,
  pitch selection, gain maps, table and polynomial mappings, vignette
  correction, and bad-pixel repair.
- `external/dnglab` at
  `62b77de1a206d467e910d55199191068b45a9cc1` was consulted only as a
  behavioral reference for integer/f32 transform boundaries.
- `flashraw-codecs` 0.1.1 (`MIT OR Apache-2.0`) was used as a temporary
  black-box parity oracle while replacing the old parser. No implementation
  source was adapted.

## Supported behavior

Fornax parses big-endian `OpcodeList1` data and applies these pixel operations
in list order:

- `FixVignetteRadial`;
- `FixBadPixelsConstant` and `FixBadPixelsList`;
- `MapTable`;
- `MapPolynomial`;
- `GainMap`;
- `DeltaPerRow` and `DeltaPerColumn`;
- `ScalePerRow` and `ScalePerColumn`.

`OpcodeList1` runs in stored sample coordinates on a single Bayer plane.
Region bounds, row and column pitch, plane selection, operation ordering, and
region clipping follow the DNG fields. Values are transformed in f32 and
rounded to the stored type only at the final boundary.

## Numerical conventions

- Table lookup rounds to the nearest index with `f32::round` semantics and
  clamps to the first or last table entry.
- Polynomial inputs and outputs use the 16-bit `[0, 65535]` domain even when
  stored `OpcodeList1` samples are 8-bit or higher precision.
- Gain maps interpolate bilinearly in normalized whole-image coordinates and
  select map planes with the DNG plane index.
- Row and column deltas are expressed in normalized 16-bit units and
  multiplied by 65535 during storage-coordinate application.
- Bad-pixel repairs average the nearest same-color Bayer neighbors at the
  fixed two-pixel CFA period.
- Vignette radius is normalized to the farthest image corner.

## Rejection policy

Geometric operations (`WarpRectilinear`, `WarpFisheye`, and `TrimBounds`) and
unknown operation IDs return a typed error instead of being silently ignored.
`OpcodeList2` and `OpcodeList3` are rejected because they apply to
linearization and demosaiced stages that Fornax does not model here.

Malformed list counts, truncated bodies, oversized element arrays, invalid
pitch zero normalization, non-finite opcode output, and out-of-range typed
output return `DecodeError::InvalidInput`.

## Fixture coverage

The available local RAW corpus did not contain a DNG directory with
`OpcodeList1` tag `51008`. The parser and every supported operation therefore
have synthetic byte-level and numerical tests; the former dependency was used
as a temporary differential oracle before removal.
