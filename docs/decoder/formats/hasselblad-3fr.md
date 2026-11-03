# Hasselblad 3FR format plan

**Tier:** 2, placeholder until this tier starts

## Container structure

3FR uses a TIFF container with Hasselblad-specific metadata and one or more raw
IFDs. The raw image is selected by dimensions, white-level presence, and CFA
metadata. Some Hasselblad files are named FFF or embed a separate raw data
area; those variants are outside this plan unless a corpus investigation adds
them explicitly.

## Compression schemes

Initial investigation targets uncompressed 16-bit samples and lossless
JPEG-based compression. Unsupported lossy, multi-shot, or proprietary
compression variants return `DecodeError::UnsupportedVariant`.

## Metadata to DNG tags

Map or preserve:

- `Make`, `Model`, serial, and lens identity;
- full dimensions, `ActiveArea`, and default crop;
- CFA repeat/pattern/plane colors;
- `BlackLevel` and `WhiteLevel`;
- `AsShotNeutral`;
- `Orientation` and safe EXIF values.

## Known variants and pitfalls

- TIFF offsets may point into vendor-specific blocks.
- White and black levels can be in separate IFDs from geometry.
- Some camera backs expose non-Bayer layouts; these remain valid decoded CFA
  data but cannot enter the preprocessor.
- FFF and merged/multi-shot files are not assumed to share the 3FR path.

## References to consult

- `external/dnglab` at `62b77de1a206d467e910d55199191068b45a9cc1`:
  `rawler/src/decoders/tfr.rs`.
- `external/libraw` at `cf02d66eb5c19d4e46df7aee0582e47a4918509a`:
  Hasselblad container and level behavior.

## Definition of done

- The accepted 3FR corpus cameras are frozen in the manifest.
- Uncompressed and accepted lossless samples decode with sane geometry and
  tags.
- Each accepted conventional Bayer image has an exact pixel comparison and a
  successful handoff; all other CFAs return `HandoffError::UnsupportedCfa`.
