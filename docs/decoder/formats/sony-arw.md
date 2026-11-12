# Sony ARW Format

This format guide defines the supported container, compression, metadata, and completion requirements for Sony ARW.

**Tier:** 1

**Status:** Implemented for uncompressed, ARW1, ARW2, ARW6, and
single-component tiled LJPEG CFA images. ARQ and reduced-resolution YCbCr
variants remain deliberately unsupported.

## Container structure

ARW and NRW files are TIFF containers with EXIF and Sony maker-note IFDs. The
raw image is selected from a CFA IFD and its strip/tile layout. Sony's
`DNGPrivateData` block points to private IFDs containing image geometry,
levels, white balance, and other camera parameters.

The reader must handle classic TIFF byte order, multiple IFDs, vendor private
offsets, and uncompressed versus tiled compressed layouts.

## Compression schemes

Accepted families:

- uncompressed 12- or 16-bit samples;
- ARW1 (`32767`) and ARW2 (`32767`) nonlinear/bit-packed compression with
  camera curves;
- ARW6 (`32766`) wavelet-style compression;
- LJPEG (`7`) tiles used by newer cameras such as the Alpha 1;
- ARQ pixel-shift/multi-sample variants are rejected until an unambiguous
  primary CFA reduction is implemented.

The ARW2 8-bit path needs camera-specific curve and black/white rescaling.
The newer LJPEG tiled path packs two Bayer lines into each decoded row and may
use a 4-component 256x256 stream for a 512x512 output tile.

`fornax-raw-sony` owns ARW1, ARW2, ARW6, and Sony lossless-JPEG entropy
decoding behind its private `ICodecDecode` codec. It also owns container
selection, Sony private-data decryption, curve-derived white levels, LLVC3
data, sample mapping, metadata conversion, and the LJPEG two-row unpack.

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

### Reduced-resolution YCbCr RAW

Sony's lossless-compressed medium and small RAW modes on recent cameras can be
stored as three-component, 15-bit-per-sample YCbCr lossless JPEG rather than a
single-sample CFA. These files use compression `7`,
`PhotometricInterpretation::YCbCr`, `SamplesPerPixel = 3`, and 512x512 LJPEG
tiles. They decode to `LinearRaw` RGB, not a Bayer mosaic.

Fornax does not synthesize a Bayer mosaic from these RGB samples and does not
broaden the current decoder contract for multi-component RAW. They therefore
return `DecodeError::Unsupported` during decoder construction and remain
expected `❌ decode:unsupported` corpus failures until an explicit linear-RGB
pipeline is designed and approved.

The corpus entries currently in this category cover:

- Sony ILCE-7M4 lossless-compressed medium/small full-frame and small APS-C;
- Sony ILCE-7CR medium and small RAW;
- Sony ILCE-9M3 medium and small RAW;
- SONY ILCE-1M2 lossless-compressed medium/small full-frame and small APS-C;
- SONY DSC-RX1RM3 medium and small RAW.

## References to consult

- `external/dnglab` at `62b77de1a206d467e910d55199191068b45a9cc1`:
  `rawler/src/decoders/arw.rs`, Sony maker-note definitions, packed and
  wavelet decompressors, and LJPEG tile handling.
- `external/libraw` at `cf02d66eb5c19d4e46df7aee0582e47a4918509a`:
  Sony levels, curves, and ARW2 behavior.

Both references are copyleft and remain behavior-only references. The
production codecs are independently implemented in Fornax. The fixed Sony
LLVC3 output table is format data checked against the permissively licensed
pre-migration dependency, as documented in
[the Sony ARW reference note](../../reference/decoders/sony-arw.md).

## Definition of done

- The accepted corpus families have fixture-backed tests and tracker rows with
  `pass=1` where fixtures are available.
- Integer samples match Rawler or LibRaw exactly after active-area, curve, CFA,
  and endianness alignment.
- Unsupported HE/reduced-resolution/ARQ variants return
  `DecodeError::Unsupported`, not corruption or panic.
- Synthetic tests cover TIFF private offsets, packed rows, tile geometry,
  LJPEG two-row unpacking, and the CFA handoff.
