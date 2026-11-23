# Architecture Overview

This document explains Fornax's crate boundaries, stage contracts, and end-to-end image-processing data flow.

## Purpose

Fornax is a Rust RAW-image processing workspace with separate decode,
preprocess, demosaic, and post-process stages. Decoder crates return a decoded
mosaic plus normalized process metadata. Preprocessing operates on the cropped
mosaic, demosaicing produces RGB output, and optional post-processing runs on
that RGB image.

The active pipeline accepts conventional 2x2 RGB Bayer input only: RGGB, BGGR,
GRBG, and GBRG. X-Trans, four-color, monochrome, and arbitrary-CFA data are not
part of the current preprocessor/demosaic handoff.

## Crate map

| Crate | Role | Internal dependencies |
| --- | --- | --- |
| `fornax-core` | Shared image aliases used by codec and raw crates. | None |
| `fornax-bmff` | Bounded generic ISO-BMFF boxes, FullBoxes, UUID payloads, and sample tables. | None |
| `fornax-tiff` | Generic TIFF tags, typed TIFF constants, values, directories, raster strip/tile planning, classic/BigTIFF parsing, and classic-TIFF writing. | `fornax-codec` |
| `fornax-codec` | Format-neutral decode and encode requests, sample buffers, chunk assembly, predictors, Huffman and byte-order support, `ICodecDecode`/`ICodecEncode`, and generic codecs. | `fornax-core` |
| `fornax-raw` | Shared raw contracts, DNG decoding/writing, native vendor decoders, preprocessing, demosaicing, post-processing, and the public facade. | `fornax-core`, `fornax-bmff`, `fornax-codec`, `fornax-tiff` |
| `fornax-cli` | Standalone batch converter for raw DNG, rendered TIFF/JPEG, and metadata JSON output. | `fornax-raw` facade |
| `fornax-raw` benches | Decoder, DNG writer, preprocessor, and demosaic comparison benchmarks. | `fornax-raw`, `fornax-devtool`, Rawler and FlashRaw dev adapters |
| `decoder-tracker` | Manual sharded raw.pixls.us decoder regression tracker and baseline generator. | `fornax-raw`, `fornax-tiff` |
| `fornax-devtool` | Generic benchmark configuration and development helpers. | None |
| `fornax-py` | PyO3/Maturin binding source, excluded from the Rust workspace and currently not buildable pending its reserved API decision. | External Python environment |

`fornax-py` requests the removed workspace dependency `fornax`,
whose current replacement is `fornax-raw`. Its source still imports
`fornax::dnc`, `fornax::fornax_dalim`, `fornax::libraw`, and
`fornax::Fornax`, none of which the current facade exports. The intended
Python API remains a
[reserved decision](../archive/workspace-refactor/steps-1-6-review.md#11-decisions-and-gaps).

## Decoder contract

`fornax_raw::protocol::IDecoder<I, T>` separates construction from access to
decoded data:

```rust
pub trait IDecoder<I, T> {
    fn decode(input: I) -> Result<Self, DecodeError>;
    fn raw_mosaic(&self) -> Result<RawMosaic, DecodeError>;
    fn cropped_mosaic(&self) -> Result<CroppedMosaic<T>, DecodeError>;
    fn process_metadata(&self) -> Result<ProcessMetadata<T>, DecodeError>;
}

pub trait IDecoderDng<I, T>: IDecoder<I, T> {
    fn dng_metadata(&self) -> Result<fornax_tiff::Directory, DecodeError>;
}
```

`raw_mosaic` exposes the decoded samples in their storage type. `RawMosaic`
holds U8, U16, U32, or F32 image buffers. `cropped_mosaic<T>` converts samples
to the requested floating-point pipeline type, crops to `ActiveArea`, and
returns a `CroppedMosaic<T>`. `process_metadata` returns geometry, levels, the
active-area-relative Bayer pattern, and optional green-normalized white-balance
coefficients.

The default implementation is `fornax_raw::Decoder`. It dispatches DNG
containers to the `fornax_raw::dng` module and vendor containers to the unconditional
`fornax_raw::{canon, sony, nikon, fujifilm, hasselblad, panasonic,
phase_one}` modules.

## Codec API

`fornax_codec::ICodecDecode` decodes one `DecodeRequest` into a single-band
`fornax_codec::SampleBuffer`. `DecodeRequest` carries source bytes, geometry,
chunk placement, sample layout, byte order, and predictor, but not an algorithm
field. Callers select the codec value and call `codec.decode(&request)`.
`SampleBuffer` owns U8, U16, U32, F32, or F64 samples in an
`image::ImageBuffer<Luma<T>, Vec<T>>`; `Luma` denotes one scalar channel rather
than necessarily luminance.

`DecodeCompression` implements `ICodecDecode` for generic Uncompressed,
Lossless JPEG, Deflate, LZW, PackBits, Lossy JPEG, JPEG XL, and Zstandard
handling. `EncodeCompression` implements the symmetric `ICodecEncode`
interface. Generic codec features still control optional dependencies.
Zstandard decoding has no level because the level is stored in the stream;
`EncodeCompression::Zstd { level }` configures encoding.

Each vendor submodule owns a private codec enum implementing `ICodecDecode` and
keeps its stream parser, parameter types, and codec tests with that format.
Sony owns ARW1, ARW2, ARW6, Sony LJPEG, curve-derived white levels, and LLVC3
data; Canon owns CRX and its CMP1/CFA types; Nikon owns packed, word, striped,
Huffman, and tree identifiers; Fujifilm owns packed/lossless and CFA layout;
Hasselblad owns predictor-8 lossless JPEG; Panasonic owns V4/V5/V6/V8, stamped
layouts, and V8 tables; Phase One owns uncompressed, lossless, nonlinear, and
Sv2 codecs. There is deliberately no Panasonic V7 implementation.

`fornax-tiff::CompressionMethod` remains the typed TIFF field value.
`fornax_tiff::RasterLayout` owns strip/tile geometry and chunk ordering, while
`fornax_tiff::RasterRequest` accepts an already-resolved codec compression.
`resolve_tiff_compression` supplies source-confirmed TIFF-family IDs and
distinguishes unknown IDs from known IDs whose codec feature is disabled.
These raster integration APIs are behind `fornax-tiff`'s non-default `raster`
feature. The feature enables its optional `fornax-codec` dependency; TIFF-only
consumers can use the parser, values, tags, and writer without codec, image, or
Rayon dependencies. `fornax-raw` enables `raster` and `jpegxl` on its TIFF
dependency. `fornax_raw::dng::constants` owns only DNG compression IDs `34892` and
`52546` plus DNG photometric values. `fornax-raw` resolves those values before
falling back to the TIFF resolver. `fornax-codec` therefore does not depend on
TIFF, DNG, or raw workflow types. `fornax-raw` adapts codec `SampleBuffer`
values to and from `fornax_raw::RawMosaic` at its decode and write
boundaries, keeping `RawMosaic` ownership unchanged.

The TIFF `raster` feature provides uncompressed, lossless JPEG-92, and Deflate
support; `jpegxl` and `zstd` forward their optional codecs. `fornax-raw` enables
TIFF JPEG XL and forwards its `zstd` feature to both TIFF and codec layers. A
known generic compression ID whose feature is disabled produces a typed
unsupported error rather than an unknown-ID error. `fornax-raw` always
maps a known vendor extension to its native submodule before any Adobe DNG
Converter fallback. The `fornax-raw` facade retains its `zstd` forwarding path.

## Pipeline flow

```text
RAW file
  -> IDecoder::decode
  -> decoder.raw_mosaic / dng_metadata
  -> decoder.process_metadata
  -> decoder.cropped_mosaic<T>
  -> CroppedMosaic<T>
  -> Preprocessor
  -> NormalizedMosaic<T>
  -> PostProcessor
  -> RgbImage<T>
```

`CroppedMosaic<T>::process` delegates to `IPreprocessor<T>`. The production
preprocessor subtracts repeating black levels and optional row/column deltas,
then normalizes by the remaining white-level range.

`NormalizedMosaic<T>::process` delegates to `IPostProcessor<T, M>`.
`fornax-raw` owns `IDemosaic<T>` and `LazyDemosaic<T>`.
`PostProcessor::default()` uses Linear demosaicing and applies green-normalized
white-balance coefficients when metadata provides them.

`ProcessMetadata<T>` carries:

- active and default-crop rectangles;
- black-level pattern, row/column deltas, and white level;
- the Bayer pattern relative to the cropped mosaic;
- optional green-normalized white-balance coefficients;
- optional DNG `ColorMatrix1`, `ColorMatrix2`, `ForwardMatrix1`, and
  `ForwardMatrix2` values;
- DNG `CameraCalibration1` and `CameraCalibration2`, represented as 3x3 matrix.
- typed provenance for selected white-balance and black-level sources.

Postprocessing can emit camera-native RGB or convert scene-linear samples to
sRGB, Adobe RGB, Wide Gamut RGB, ProPhoto RGB, XYZ, ACES2065-1, ACEScg, DCI-P3,
or Rec. 2020. The default remains camera-native RGB.

`default_crop` is a recommendation. Stage methods do not apply it automatically.

## DNG support

Detailed storage, metadata, and acceptance behavior lives in the
[DNG decoder reference notes](../reference/decoders/reference/dng-decoder.md).

The default decoder reads classic TIFF and BigTIFF directories, selects the
full-resolution CFA or LinearRaw image, preserves typed root and raw-IFD
metadata, and decodes uncompressed, lossless JPEG-92, Deflate, lossy JPEG, and
JPEG XL raw chunks. Independent strips and tiles are decoded with Rayon. Raw
storage transforms, independently implemented `OpcodeList1` pixel operations,
and linearization are applied before returning the mosaic. Its
process-metadata conversion accepts only conventional 2x2 RGB Bayer layouts.
`OpcodeList2` and `OpcodeList3` are ignored by the raw mosaic decoder because
their stage-specific transforms are not applied.

Compression `52546` is the DNG 1.7 JPEG XL identifier and compression `50002`
is the self-assigned TIFF JPEG XL identifier. Both map to the single-channel
decoder.
Multi-channel `LinearRaw` and three-component lossy JPEG remain outside the
current mono mosaic decoder contract. Multi-component lossless-JPEG tile
variants remain an unsupported codec layout within that contract.

`fornax_raw::writer::DngWriter` regenerates uncompressed or lossless DNG data from
`RawMosaic` and a `fornax_tiff::Directory`. The lossless option writes tiled JPEG-92 for
U8/U16 samples and a lossless Deflate representation for U32/F32 samples.
Regeneration is not byte-identical to the source because the public model does
not retain the original container or auxiliary directories.

## Sony ARW/NRW support

`fornax_raw::sony` selects the full-resolution CFA IFD from classic TIFF or BigTIFF,
parses and decrypts Sony `DNGPrivateData`, maps geometry, CFA, black/white
levels, white balance, and crop data into the shared DNG metadata model, and
adapts strips or tiles to `fornax-codec`. It supports uncompressed 12-bit
packed and 14/16-bit word samples, ARW1, ARW2, ARW6, and tiled LJPEG.
Four-component Sony LJPEG tiles are unpacked by the codec into two Bayer output
rows per JPEG row.

Reduced-resolution YCbCr/`LinearRaw` images, multi-sample ARQ files, unknown
compression families, and non-Bayer CFAs return typed unsupported errors.
Camera-specific color matrices are not synthesized when the source container
does not provide them.

## Fujifilm RAF support

`fornax_raw::fujifilm` parses the RAF preview, proprietary tag, and Fuji TIFF pointers;
the type-less RAF tag table; and the embedded Fuji IFD. It supports
uncompressed 12-bit packed and 14/16-bit word samples plus Fujifilm 12/14/16-bit
block-lossless compression.

Standard 2x2 GFX Bayer files use explicit camera profiles for CFA and sensor
active-area borders. X-Trans layouts use the 36-plane RAF tag and remain a
general 6x6 CFA through `raw_mosaic` and DNG writing. The shared
`process_metadata` and `cropped_mosaic` boundary rejects X-Trans with a typed
unsupported error, so X-Trans never reaches preprocessing or demosaicing.
Lossy RAF, SuperCCD rotation/staggering, FinePix DBP, unknown Bayer models, and
unknown compression families are also rejected with typed unsupported errors.

## Hasselblad 3FR support

`fornax_raw::hasselblad` selects the full-resolution CFA SubIFD from a classic
little-endian TIFF container, fills absent CFA tags from explicit RGGB camera
profiles, and decodes either exact 16-bit little-endian words or predictor-8
lossless JPEG. The latter applies two leading Huffman lengths per pixel pair,
per-row predictor resets, the 3FR four-byte bit order, and class-16 payload
consumption.

Source `ColorMatrix1`, `AsShotNeutral`, levels, crop, identity, orientation, and
safe Exif values are mapped into the shared DNG metadata directory. X1D-family
profiles provide the inactive sensor borders. Well-formed non-2x2 CFA data can
be exported through DNG, but the preprocessing handoff rejects it. FFF,
merged/multi-shot, tiled, unknown tagless models, and unknown compression
families return typed unsupported errors.

## Canon CR3/CRM support

`fornax_raw::canon` parses CR3 and CRM BMFF containers, selects CRAW image type zero
from CCTP or a unique CRAW track, parses CMT1-CMT4 through `fornax-tiff`, and
reads ColorData from the CTMD sample. Lossless CRX samples decode to a storage-
native U16 mosaic with IAD1 active area and crop, Bayer placement, black/white
levels, and as-shot neutral metadata.

C-RAW, preview and JPEG tracks, ambiguous raw tracks, non-2x2 layouts, unknown
ColorData versions, and unsupported CRX header shapes return typed errors
before preprocessing. CR3 and CRM both require the `crx` compatible brand; the
filename extension selects `RawFormat::Cr3` or `RawFormat::Crm`.

## Phase One/Leaf IIQ support

`fornax_raw::phase_one` parses the TIFF container followed by the Phase One private header
at file offset 8. Private maker-note and raw-data offsets are relative to that
base. The decoder supports compression modes `0`, `3`, `5`, `6`, and `8`;
legacy Raw1/Raw2 modes return typed unsupported errors.

Decoded U16 samples are processed through split black levels, polynomial
curves, quadrant calibration, bad-column repair, and flat-field correction
before `raw_mosaic` returns. Corrected output uses DNG `BlackLevel=0` and
`WhiteLevel=65535`. Selected corpus fixtures match Rawler sample-for-sample
after correction.

Explicit Phase One and Leaf camera profiles supply conventional 2x2 CFA
layouts. The source maker note supplies dimensions, margins, crop size,
white-balance gains, and correction payloads. Unknown camera models return
typed unsupported errors rather than guessed CFA data. Leaf `.MOS`, Phase One
`.TIF`, preview extraction, and non-Bayer pipeline input remain outside the
implemented boundary.

## Panasonic RW2 support

`fornax_raw::panasonic` parses classic-TIFF files with magic 85, selects the
first primary IFD with Panasonic `RawDataOffset` and falls back to
`StripOffsets`, then dispatches raw-format V4, V5, V6, and V8 through its local
codec. Stamped 12-bit stripped layouts are selected by payload size.

The format layer maps border coordinates to `ActiveArea`, shifts CFA into the
active reference frame, maps exclusive crop coordinates to DNG default crop,
applies the historic V4 black offset, reduces Panasonic linearity limits to
one white level, normalizes as-shot neutral, and preserves raw-IFD orientation
plus Exif serial/lens when available. The result round-trips through
`DngWriter`.

Multi-shot tags, high-resolution composites, V7, unknown versions or
compression values, and unsupported CFA layouts return typed unsupported
errors before sample decoding. V7 remains deferred until a real single-shot
file can validate the codec.

## Benchmarks and comparisons

Decoder benchmarks live in `crates/fornax-raw/benches/decoder.rs`. The
benchmark compares Fornax, Rawler, and FlashRaw over the decoder-tracker golden
corpus with one Criterion group per input. The shared measurements are
`decode`, `raw_mosaic`, and `cropped_mosaic`; Fornax metadata remains
diagnostic-only. `f32` is the default sample type, and `f64` and decoder
subsets are opt-in. Preprocessor and demosaic benchmarks live in
`fornax-raw`.

`decoder-tracker` validates the pointer-only raw.pixls.us submodule manifest and
runs manual, deterministically sharded decoder regressions against a committed
CSV baseline. RAW payloads are transient and never uploaded or committed.

See [the decoder-tracker agent guide](../../crates/decoder-tracker/AGENTS.md) for
corpus setup, run, baseline-update, and README rules.

## References

- Decoder details: [docs/architecture/decoder.md](decoder.md)
- Decoder benchmark plan:
  [docs/reference/decoders/operations/benchmark.md](../reference/decoders/operations/benchmark.md)
- Decoder corpus plan:
  [docs/reference/decoders/operations/corpus.md](../reference/decoders/operations/corpus.md)
