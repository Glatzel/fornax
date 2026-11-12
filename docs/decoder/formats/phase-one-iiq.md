# Phase One IIQ Format

This format guide defines the supported container, compression, metadata, and completion requirements for Phase One IIQ.

**Tier:** 2, implemented for Phase One/Leaf `.IIQ` modes 0, 3, 5, 6, and 8

## Container structure

IIQ starts with a classic little-endian TIFF header whose first IFD and preview
resources normally sit near the end of the file. A Phase One private header
starts at file offset 8. It contains the `Raw` signature, a private IFD offset,
and maker-note entries whose value and payload offsets are relative to that
private base rather than the file start.

The private dimensions describe the stored sensor raster. Margin and image
size tags define `ActiveArea`; the raw payload, per-row offsets, white-balance
gains, black level, split-black tables, and sensor-correction directory are
separate maker-note values.

## Compression schemes

| Mode | Meaning | State |
| --- | --- | --- |
| `0` | Uncompressed 14-bit little-endian words, scaled to U16 | Implemented |
| `3` | IIQ-L 14-bit row-difference coding | Implemented |
| `5` | IIQ-S nonlinear low-value coding | Implemented |
| `6` | IIQ-Sv2 eight-pixel cluster coding | Implemented |
| `8` | IIQ-L16 16-bit row-difference coding | Implemented |
| `1`, `2` | Legacy Raw1/Raw2 | Typed unsupported |

The compressed modes use a per-row offset table whose entries are relative to
the raw payload. Fornax sorts offsets by file position, rejects duplicate or
empty ranges, and supplies one validated codec chunk per output row. IIQ-L/L16
uses alternating even/odd predictors with per-eight-pixel length selectors.
IIQ-S applies the fixed low-value nonlinear curve. IIQ-Sv2 reconstructs
eight-pixel clusters and applies bounded error recovery.

## Sensor correction

`raw_mosaic` returns corrected U16 samples. Correction order is:

1. base black and signed split-column/split-row black offsets;
2. half-sensor and full-sensor polynomial curves;
3. quadrant multipliers, combined gain curves, and quadrant linearization;
4. bad-column repair;
5. all-color and red/blue flat-field corrections.

Supported correction tags are `0x0400`, `0x0401`, `0x040b`, `0x0410`, `0x0416`,
`0x0419`, `0x041a`, `0x041e`, `0x041f`, and `0x0431`. Unrelated correction
payload tags observed in valid files are ignored. Malformed supported payloads
return typed input errors.

Because output is already corrected, exported DNG metadata has
`BlackLevel=0` and `WhiteLevel=65535`. Rawler `0.8.0` is the development-only
pixel oracle. P20+, H25, Credo 40 IIQ-L/Sv2, and IQ3 100MP IIQ-L16 selected
corpus files match Rawler sample-for-sample.

## Metadata mapping

Fornax maps or preserves:

- Make, Model, `UniqueCameraModel`, and valid TIFF orientation;
- full dimensions, sensor margins, `ActiveArea`, and default crop;
- 2x2 `CFARepeatPatternDim`, `CFAPattern`, and `CFAPlaneColor`;
- corrected black level and U16 white level;
- `AsShotNeutral` from reciprocal green-normalized IIQ white-balance gains.

Explicit profiles cover the Phase One and Leaf models represented in the pinned
IIQ corpus with conventional 2x2 RGB CFA layouts. Unknown models return typed
unsupported errors rather than guessed CFA data. Camera color matrices are not
synthesized when the source does not provide a validated DNG-compatible matrix.

## Known variants and pitfalls

- Maker-note offsets are relative to file offset 8, not the TIFF or file base.
- The bitstream is read as little-endian 32-bit words but consumed MSB-first.
- IIQ-S high values are unshifted while low values use the nonlinear U16 curve.
- Flat-field CFA lookup is relative to the configured sensor margins.
- P65+ files with a 260-byte quadrant-linearization payload use an unsupported
  variant and return a typed error instead of applying a guessed curve.
- Defect and flat-field correction are capture- or model-specific and must not
  be approximated with guessed data.
- Leaf `.MOS`, Phase One `.TIF`, preview extraction, legacy Raw1/Raw2, and
  non-2x2 CFA pipeline input remain unsupported.

## References

- dnglab/Rawler at
  `62b77de1a206d467e910d55199191068b45a9cc1`:
  `rawler/src/decoders/iiq.rs` and `rawler/src/imgop/spline.rs`.
- LibRaw at `cf02d66eb5c19d4e46df7aee0582e47a4918509a`:
  `src/metadata/mediumformat.cpp`, `src/decoders/load_mfbacks.cpp`,
  `src/decoders/decoders_libraw.cpp`, and
  `src/utils/phaseone_processing.cpp`.
- Both are LGPL-2.1 and remain behavioral references only. See
  [Phase One IIQ references](../../reference/decoders/phase-one-iiq.md).

## Definition of done

- Modes 0, 3, 5, 6, and 8 have synthetic or corpus coverage.
- Accepted compressed fixtures match Rawler after correction.
- Legacy/unknown modes and unknown camera profiles return typed errors.
- Corrected samples and 2x2 CFA metadata survive DNG write/read-back.
- The default facade routes `.IIQ`, the full IIQ corpus baseline is reviewed,
  golden hashes are generated, and workspace validation passes.
