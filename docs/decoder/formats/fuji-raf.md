# Fujifilm RAF format plan

**Tier:** 2, placeholder until this tier starts

## Container structure

RAF starts with a `FUJIFILM` header and pointer fields to one or more
TIFF-like metadata structures and proprietary RAF data. The raw image has a
separate byte offset and byte count; the embedded JPEG is a preview, not the
raw payload. Fujifilm's RAF IFD omits normal type metadata for several tags, so
the parser must use a format-specific schema.

## Compression schemes

Targets, contingent on corpus inventory:

- uncompressed packed 12-, 14-, or 16-bit samples;
- Fujifilm block-based lossless compression;
- older SuperCCD packed/rotated layouts.

Some lossy RAF variants are outside the initial target and should return a
typed unsupported-variant result.

## Metadata to DNG tags

Map or preserve:

- `Make`, `Model`, and unique camera identity;
- full raw dimensions and crop origin/size;
- `CFARepeatPatternDim`, `CFAPattern`, `CFAPlaneColor`, and `CFALayout`;
- `BlackLevel`, `WhiteLevel`, and level offsets;
- `AsShotNeutral`/Fujifilm white-balance levels;
- `Orientation` and safe EXIF values.

X-Trans uses a 6x6 repeat. It must remain a general CFA through decode and DNG
export and must fail only at `DecodedRaw::preprocessor_pattern`.

## Known variants and pitfalls

- X-Trans CFA layout must be read from file metadata and is not always the same
  across compressed and uncompressed modes.
- Some SuperCCD sensors require a 45-degree rotation or non-rectangular
  staggered layout; preserve `CFALayout` and do not flatten it into a 2x2
  pattern.
- Strip byte counts may be absent; bounded reads to EOF are allowed only when
  the declared raw dimensions prove the required length.
- RAF's proprietary blocks are not ordinary TIFF and need strict entry-count
  limits.
- The embedded JPEG may contain the XMP packet but is not a raw fallback.

## References to consult

- `external/dnglab` at `62b77de1a206d467e910d55199191068b45a9cc1`:
  `rawler/src/decoders/raf.rs` and `raf/fuji_decompressor.rs`.
- `external/libraw` at `cf02d66eb5c19d4e46df7aee0582e47a4918509a`:
  Fujifilm decompression and SuperCCD behavior.

## Definition of done

- The corpus defines the accepted RAF camera/compression set.
- X-Trans decode plus DNG export passes sample/tag round-trip tests.
- Every X-Trans handoff returns `HandoffError::UnsupportedCfa`; no X-Trans
  image reaches the preprocessor.
- Supported conventional 2x2 Fujifilm files pass the Bayer handoff.
