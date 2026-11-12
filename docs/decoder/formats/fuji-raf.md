# Fujifilm RAF Format

This format guide defines the supported container, compression, metadata, and completion requirements for Fujifilm RAF.

**Tier:** 2, implemented for uncompressed and Fujifilm block-lossless images

## Container structure

RAF starts with a `FUJIFILM` header and pointer fields to one or more
TIFF-like metadata structures and proprietary RAF data. The raw image has a
separate byte offset and byte count; the embedded JPEG is a preview, not the
raw payload. Fujifilm's RAF IFD omits normal type metadata for several tags, so
the parser must use a format-specific schema.

## Compression schemes

Implemented targets:

- uncompressed 12-bit little-endian packed samples;
- uncompressed 14- and 16-bit word samples;
- Fujifilm block-based lossless compression at 12, 14, and 16 bits.

Lossy RAF variants, older SuperCCD packed/rotated layouts, and FinePix DBP
layouts return typed unsupported-variant errors.

## Metadata to DNG tags

Map or preserve:

- `Make`, `Model`, and unique camera identity;
- full raw dimensions and crop origin/size;
- `CFARepeatPatternDim`, `CFAPattern`, `CFAPlaneColor`, and `CFALayout`;
- `BlackLevel`, `WhiteLevel`, and level offsets;
- `AsShotNeutral`/Fujifilm white-balance levels;
- `Orientation` and safe EXIF values.

X-Trans uses a 6x6 repeat. It must remain a general CFA through raw-mosaic
decoding and DNG export, and must fail only when `IDecoder::cropped_mosaic` or
process-metadata conversion tries to construct conventional Bayer pipeline
input.

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

- The RAF corpus records supported lossless Bayer files and expected
  X-Trans, lossy, and legacy outcomes.
- X-Trans decode plus DNG export passes sample and 6x6 CFA round-trip tests.
- Every X-Trans handoff returns a typed unsupported-CFA error before the
  preprocessor.
- Supported conventional 2x2 GFX files pass the Bayer handoff.
