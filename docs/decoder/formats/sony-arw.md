# Sony ARW format plan

**Tier:** 1

## Container structure

ARW and NRW files are TIFF containers with EXIF and Sony maker-note IFDs. The
raw image is selected from a CFA IFD and its strip/tile layout. Sony's
`DNGPrivateData` block points to private IFDs containing image geometry,
levels, white balance, and other camera parameters.

The reader must handle classic TIFF byte order, multiple IFDs, vendor private
offsets, and uncompressed versus tiled compressed layouts.

## Compression schemes

Reader targets:

- uncompressed 12- or 16-bit samples;
- ARW1 (`32767`) and ARW2 (`32767`) nonlinear/bit-packed compression with
  camera curves;
- ARW6 (`32766`) wavelet-style compression;
- LJPEG (`7`) tiles used by newer cameras such as the Alpha 1;
- ARQ pixel-shift/multi-sample variants only when the primary raw sample can
  be reduced safely to one CFA mosaic.

The ARW2 8-bit path needs camera-specific curve and black/white rescaling.
The newer LJPEG tiled path packs two Bayer lines into each decoded row and may
use a 4-component 256x256 stream for a 512x512 output tile.

## Metadata to DNG tags

Map or preserve:

- `Make`, `Model`, `UniqueCameraModel`, serial and lens identity;
- `ActiveArea` from `SonyRawImageSize` or camera fallback;
- `DefaultCropOrigin`, `DefaultCropSize`, and `Orientation`;
- `CFARepeatPatternDim`, `CFAPattern`, `CFAPlaneColor`;
- `BlackLevelRepeatDim`, `BlackLevel`, `WhiteLevel`;
- `AsShotNeutral` or normalized Sony white-balance coefficients;
- Sony curve/level data that affects sample interpretation;
- safe EXIF and maker-note values.

Curve-applied samples require the resulting white level to match the decoded
sample stage. Keep the curve values in metadata when export must reproduce the
same interpretation.

## Known variants and pitfalls

- The same TIFF compression number has different Sony-specific meanings.
- Some reduced-resolution or YCbCr samples are `LinearRaw`, not CFA; they must
  not enter the Bayer handoff.
- LJPEG tile geometry can differ from TIFF tile geometry.
- `DNGPrivateData` offsets are relative to structures that are not always
  obvious from the outer TIFF.
- Some samples need the height extended by eight rows when compressed byte
  counts imply padding.
- Camera profiles vary black level, white level, CFA phase, crop, and curve
  even within one ARW compression family.
- ARQ files may contain multiple raw samples; support only the unambiguous
  primary mosaic initially.

## References to consult

- `external/dnglab` at `62b77de1a206d467e910d55199191068b45a9cc1`:
  `rawler/src/decoders/arw.rs`, Sony maker-note definitions, packed and
  wavelet decompressors, and LJPEG tile handling.
- `external/libraw` at `cf02d66eb5c19d4e46df7aee0582e47a4918509a`:
  Sony levels, curves, and ARW2 behavior.

Both references are copyleft; use them only as behavior references under the
architecture's licence policy.

## Definition of done

- Each accepted ARW compression family and supported camera family has a
  corpus row with `pass=1`.
- Integer samples match Rawler or LibRaw exactly after active-area, curve, CFA,
  and endianness alignment.
- Unsupported HE/reduced-resolution/ARQ variants return
  `DecodeError::UnsupportedVariant`, not corruption or panic.
- Synthetic tests cover TIFF private offsets, packed rows, tile geometry,
  LJPEG two-row unpacking, and the CFA handoff.
