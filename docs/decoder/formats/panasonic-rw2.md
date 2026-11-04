# Panasonic RW2 format plan

**Tier:** 2, placeholder until this tier starts

## Container structure

RW2 is TIFF-like but uses Panasonic-specific offsets, dimensions, raw-format
and compression tags. Raw data can be addressed through Panasonic offset tags
or a conventional strip offset. A camera metadata IFD can contain lens and
level information.

## Compression schemes

The initial inventory must distinguish:

- uncompressed 12-bit left-aligned samples;
- packed 12-bit samples;
- Panasonic raw-format versions 4 through 8;
- uncompressed and compressed multi-shot/high-resolution variants.

Unknown raw-format numbers return a typed unsupported variant. They must not
reach a `todo!()` or generic panic.

## Metadata to DNG tags

Map or preserve:

- `Make`, `Model`, serial, lens identity;
- `ActiveArea` from sensor border tags and default crop;
- `CFARepeatPatternDim`, `CFAPattern`, `CFAPlaneColor`;
- `BlackLevel` including the historic +15 offset rule;
- `WhiteLevel`;
- `AsShotNeutral` from Panasonic white-balance tags;
- `Orientation` and EXIF values.

## Known variants and pitfalls

- A black-level offset of 15 applies when the raw-data-offset tag is absent or
  the raw format is old.
- Width/height tags may describe a crop mode rather than the full sensor.
- The file extension is not enough to identify a raw format version.
- High-resolution/multi-shot files may contain more than one exposure or a
  reconstructed sample layout.
- Sample count can be inferred from remaining data length only after checking
  all declared dimensions.

## References to consult

- `external/dnglab` at `62b77de1a206d467e910d55199191068b45a9cc1`:
  `rawler/src/decoders/rw2.rs` and RW2 v4-v8 decompressor modules.
- `external/libraw` at `cf02d66eb5c19d4e46df7aee0582e47a4918509a`:
  Panasonic level and compression behavior.

## Definition of done

- Every accepted RW2 camera/raw-format combination has a corpus row.
- Supported packed/unpacked samples match the reference after crop, black
  offset, and CFA alignment.
- Multi-shot unsupported variants return the typed variant error.
- Conventional Bayer files pass the handoff; any nonstandard CFA is rejected
  with `HandoffError::UnsupportedCfa`.
