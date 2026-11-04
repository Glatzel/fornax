# Phase One IIQ format plan

**Tier:** 2, placeholder until this tier starts

## Container structure

IIQ is a TIFF container followed by a Phase One maker-note directory whose
offsets are relative to a private base. The maker note identifies image
dimensions, sensor margins, raw/compressed data and strip offsets, white
balance, black level, and optional sensor-correction data.

## Compression schemes

Known modes:

| Mode | Meaning |
| --- | --- |
| `0` | Uncompressed |
| `3` | IIQ-L |
| `5` | IIQ-S |
| `6` | IIQ-Sv2 |
| `8` | IIQ-L16 |
| `1`, `2` | Legacy Raw1/Raw2, not initial targets |

IIQ-S requires a nonlinear lookup curve. IIQ-Sv2 has strip-based decoding and
error recovery. Sensor-correction/flat-field processing can change sample
values and must be reflected in the exported pixel stage or preserved as
metadata.

## Metadata to DNG tags

Map or preserve:

- `Make`, `Model`, serial, and lens identity;
- full dimensions, sensor margins, `ActiveArea`, and default crop;
- CFA repeat/pattern/plane colors;
- `BlackLevel`, `WhiteLevel`;
- `AsShotNeutral`;
- sensor correction, defect, split-black, and flat-field data where safe;
- `Orientation` and EXIF values.

If sensor correction is applied while decoding, reset black levels only when
the output samples are truly corrected and document the stage in metadata.

## Known variants and pitfalls

- Maker-note offsets are relative to a private header, not always to the file
  start.
- Split-column/row black offsets are signed values.
- Defect and flat-field correction can be camera- or capture-specific.
- Legacy Raw1/Raw2 modes are not covered by the initial implementation.
- Some Leaf-branded files share the IIQ extension but not all metadata
  semantics.

## References to consult

- `external/dnglab` at `62b77de1a206d467e910d55199191068b45a9cc1`:
  `rawler/src/decoders/iiq.rs`.
- `external/libraw` at `cf02d66eb5c19d4e46df7aee0582e47a4918509a`:
  Phase One and Leaf IIQ behavior.

## Definition of done

- Accepted IIQ modes and camera families are frozen in the corpus manifest.
- Uncompressed and implemented compressed modes have exact pixel comparisons
  after correction-stage alignment.
- Legacy unsupported modes and unsupported sensor-correction variants return
  typed errors.
- Every accepted non-Bayer layout exports as DNG and is rejected only at the
  preprocessor handoff.
