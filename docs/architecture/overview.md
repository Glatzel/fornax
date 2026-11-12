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
| `fornax-raw-core` | Shared image wrappers, process metadata, errors, and stage traits. | None |
| `fornax-bmff` | Bounded generic ISO-BMFF boxes, FullBoxes, UUID payloads, and sample tables. | `fornax-raw-core` |
| `fornax-tiff` | Generic TIFF tags, typed TIFF constants, values, directories, raster strip/tile planning, classic/BigTIFF parsing, and classic-TIFF writing. | `fornax-codec` |
| `fornax-codec` | Format-neutral raw sample decoding, chunk assembly, predictors, supported codec encoding, and feature-gated Sony, Nikon, Fujifilm, Hasselblad, Canon, Phase One, and Panasonic codecs. | None |
| `fornax-dng` | DNG-only tags, raw transforms, process-metadata conversion, and DNG writing over `fornax_tiff::Directory`. | `fornax-raw-core`, `fornax-tiff`, `fornax-codec` |
| `fornax-arw` | Native Sony ARW/NRW TIFF, private metadata, CFA layout mapping, and DNG-compatible metadata mapping. | `fornax-raw-core`, `fornax-tiff`, `fornax-dng`, `fornax-codec` |
| `fornax-cr3` | Canon CR3/CRM BMFF, CMT/CTMD metadata, CRAW/CMP1/IAD1 parsing, and lossless CRX handoff. | `fornax-raw-core`, `fornax-bmff`, `fornax-tiff`, `fornax-dng`, `fornax-codec` |
| `fornax-nef` | Native Nikon NEF TIFF/maker-note parsing, standard packed CFA layouts, metadata mapping, and Bayer handoff. | `fornax-raw-core`, `fornax-tiff`, `fornax-dng`, `fornax-codec` |
| `fornax-raf` | Native Fujifilm RAF parsing, lossless Bayer/X-Trans decoding, metadata mapping, and DNG preservation. | `fornax-raw-core`, `fornax-tiff`, `fornax-dng`, `fornax-codec` |
| `fornax-hasselblad` | Native Hasselblad 3FR TIFF/SubIFD parsing, camera profiles, uncompressed and predictor-8 decoding, metadata mapping, and DNG preservation. | `fornax-raw-core`, `fornax-tiff`, `fornax-dng`, `fornax-codec` |
| `fornax-iiq` | Native Phase One/Leaf IIQ TIFF/private maker-note parsing, sensor correction, metadata mapping, and Bayer handoff. | `fornax-raw-core`, `fornax-tiff`, `fornax-dng`, `fornax-codec` |
| `fornax-rw2` | Native Panasonic RW2 TIFF layout, metadata, V4/V5/V6/V8 codecs, and Bayer handoff. | `fornax-raw-core`, `fornax-tiff`, `fornax-dng`, `fornax-codec` |
| `fornax-decoder` | Default pure-Rust DNG, Canon CR3/CRM, Sony ARW/NRW, Nikon NEF, Fujifilm RAF, Hasselblad 3FR, Phase One/Leaf IIQ, and Panasonic RW2 decoder facade over the format crates. | `fornax-raw-core`, `fornax-dng`, `fornax-cr3`, `fornax-arw`, `fornax-nef`, `fornax-raf`, `fornax-hasselblad`, `fornax-iiq`, `fornax-rw2` |
| `fornax-decoder-rawler` | Alternative Rawler-backed decoder for supported 2x2 RGB Bayer RAW files. | `fornax-raw-core`, `fornax-dng`, `fornax-tiff` |
| `fornax-decoder-dnc` | Windows/macOS adapter that converts with Adobe DNG Converter, then decodes through the Rawler adapter. | `fornax-raw-core`, `fornax-decoder-rawler` |
| `fornax-raw-pipeline` | Black/white normalization, demosaic protocol and algorithms, RGB reconstruction, and color processing. | `fornax-raw-core` |
| `fornax-raw` | Public facade over decoders and pipeline stages. | Raw core, decoder crates, raw pipeline |
| `fornax-cli` | Standalone batch converter for raw DNG, rendered TIFF/JPEG, and metadata JSON output. | `fornax-raw` facade |
| `fornax-decoder` benches | Decoder comparison benchmarks. | Core, decoder crates, devtool |
| `decoder-tracker` | Manual sharded raw.pixls.us decoder regression tracker and baseline generator. | Core, decoder, TIFF/DNG metadata |
| `fornax-devtool` | Pinned fixture paths, synthetic data, and benchmark configuration. | `fornax-raw-core` |
| `compare-demosaic` | Reference-versus-Fornax demosaic comparison harness. | Core, decoder, processors, devtool, pinned C/C++ references |
| `fornax-py` | PyO3/Maturin binding source, excluded from the Rust workspace and currently not buildable pending its reserved API decision. | External Python environment |

`fornax-py` requests the removed workspace dependency `fornax`,
whose current replacement is `fornax-raw`. Its source still imports
`fornax::dnc`, `fornax::fornax_dalim`, `fornax::libraw`, and
`fornax::Fornax`, none of which the current facade exports. The intended
Python API remains a
[reserved decision](../_archive/workspace-refactor/steps-1-6-review.md#11-decisions-and-gaps).

## Decoder contract

`fornax_raw_core::protocol::IDecoder<I, T>` separates construction from access to
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

The default implementation is `fornax_decoder::Decoder`. The facade exports it
as `fornax_raw::Decoder`; it dispatches DNG containers to `fornax-dng`, Canon
CR3/CRM containers to `fornax-cr3`, Sony ARW/NRW containers to `fornax-arw`,
Nikon NEF containers to `fornax-nef`, Fujifilm RAF containers to `fornax-raf`,
Hasselblad 3FR containers to `fornax-hasselblad`, Panasonic RW2 containers to
`fornax-rw2`, and Phase One/Leaf IIQ containers to `fornax-iiq`.
`fornax_raw_core` contains no decoder implementation.

## Codec API

`fornax_codec::DecodeCompression` and
`fornax_codec::EncodeCompression` select algorithms inside `DecodeRequest` and
`EncodeRequest`, respectively. The free `fornax_codec::decode` and
`fornax_codec::encode` functions dispatch directly to the selected
implementation and exchange a single-band `fornax_codec::SampleBuffer`.
`SampleBuffer` owns U8, U16, U32, F32, or F64 samples in an
`image::ImageBuffer<Luma<T>, Vec<T>>`; `Luma` denotes one scalar channel rather
than necessarily luminance. Zstandard decoding has no level because the level
is stored in the stream; `EncodeCompression::Zstd { level }` configures
encoding.

The non-default `arw` feature adds Sony `Arw1`, `Arw2`, `Arw6`, and
`SonyLosslessJpeg` decode variants. ARW2 carries optional four-knot
`SonyRawCurve` data; `None` selects the identity curve. The Sony variants use
the same chunk, geometry, precision, and `SampleBuffer` contract as other
codecs. `arw2_white_level` and `arw6_white_level` expose the curve-derived
white levels needed by the Sony format layer.

The non-default `nef` feature adds Nikon packed-row, unpacked-word, and
mode-5 striped packed decoding. `fornax-nef` enables this feature for its
vendor codec request.

The non-default `raf` feature adds Fujifilm direct-packed and block-lossless
decoding. `FujiCfa` selects the 2x2 or 6x6 CFA placement used when decoded
six-line buffers are written to the output mosaic.

The non-default `hasselblad` feature adds predictor-8 lossless-JPEG decoding
for 3FR mosaics. It uses the 3FR four-byte bit order, consumes the class-16
payload, and emits storage-native U16 samples.

The non-default `cr3` feature adds the lossless Canon CRX path. It accepts only
the audited CMP1 tuple (version `0x0100`, four planes, encoding type zero, zero
wavelet levels, MDAT header size `0x70`) and conventional 2x2 Bayer
placements. C-RAW and every ambiguous shape return a typed unsupported error.

The non-default `iiq` feature adds Phase One IIQ uncompressed, IIQ-L/L16,
IIQ-S nonlinear, and IIQ-Sv2 row decoding. The IIQ format crate supplies one
chunk per compressed row and performs sensor correction after codec output.
The independent non-default `rw2` feature adds Panasonic V4, V5, V6, and V8
decoding plus stamped 12-bit unpacked and packed-control layouts. V8 carries
validated gamma, Huffman, shift, and strip tables in
`PanasonicV8Params`. There is deliberately no V7 variant.

`fornax-tiff::CompressionMethod` remains the typed TIFF field value.
`fornax_tiff::RasterLayout` owns strip/tile geometry and chunk ordering, while
`fornax_tiff::RasterRequest` accepts an already-resolved codec compression.
`resolve_tiff_compression` supplies source-confirmed TIFF-family IDs and
distinguishes unknown IDs from known IDs whose codec feature is disabled.
These raster integration APIs are behind `fornax-tiff`'s non-default `raster`
feature. The feature enables its optional `fornax-codec` dependency; TIFF-only
consumers can use the parser, values, tags, and writer without codec, image, or
Rayon dependencies. `fornax-dng` enables `raster` and `jpegxl` on its TIFF
dependency. `fornax_dng::constants` owns only DNG compression IDs `34892` and
`52546` plus DNG photometric values. `fornax-dng` resolves those values before
falling back to the TIFF resolver. `fornax-codec` therefore does not depend on
TIFF, DNG, or raw workflow types. `fornax-dng` adapts codec `SampleBuffer`
values to and from `fornax_raw_core::RawMosaic` at its decode and write
boundaries, keeping `RawMosaic` ownership unchanged.

Codec availability follows Cargo features. The non-default codec `arw` feature
enables Sony ARW1, ARW2, ARW6, and lossless-JPEG tile decoding and implies
`lossless-jpeg`. The TIFF `raster` feature provides uncompressed, lossless
JPEG-92, and Deflate support; `jpegxl` and `zstd` forward their optional
codecs. `fornax-dng` enables TIFF JPEG XL and forwards its `zstd` feature to
both TIFF and codec layers. A known compression ID whose feature is disabled
produces a typed unsupported error rather than an unknown-ID error. The
`fornax-raw` facade retains its `zstd` feature forwarding path.

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
`fornax-raw-pipeline` owns `IDemosaic<T>` and `LazyDemosaic<T>`.
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

`fornax_dng::DngWriter` regenerates uncompressed or lossless DNG data from
`RawMosaic` and a `fornax_tiff::Directory`. The lossless option writes tiled JPEG-92 for
U8/U16 samples and a lossless Deflate representation for U32/F32 samples.
Regeneration is not byte-identical to the source because the public model does
not retain the original container or auxiliary directories.

## Sony ARW/NRW support

`fornax-arw` selects the full-resolution CFA IFD from classic TIFF or BigTIFF,
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

`fornax-raf` parses the RAF preview, proprietary tag, and Fuji TIFF pointers;
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

`fornax-hasselblad` selects the full-resolution CFA SubIFD from a classic
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

`fornax-cr3` parses CR3 and CRM BMFF containers, selects CRAW image type zero
from CCTP or a unique CRAW track, parses CMT1-CMT4 through `fornax-tiff`, and
reads ColorData from the CTMD sample. Lossless CRX samples decode to a storage-
native U16 mosaic with IAD1 active area and crop, Bayer placement, black/white
levels, and as-shot neutral metadata.

C-RAW, preview and JPEG tracks, ambiguous raw tracks, non-2x2 layouts, unknown
ColorData versions, and unsupported CRX header shapes return typed errors
before preprocessing. CR3 and CRM both require the `crx` compatible brand; the
filename extension selects `RawFormat::Cr3` or `RawFormat::Crm`.

## Phase One/Leaf IIQ support

`fornax-iiq` parses the TIFF container followed by the Phase One private header
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

`fornax-rw2` parses classic-TIFF files with magic 85, selects the first primary
IFD with Panasonic `RawDataOffset` and falls back to `StripOffsets`, then
dispatches raw-format V4, V5, V6, and V8 through the codec's independent `rw2`
feature. Stamped 12-bit stripped layouts are selected by payload size.

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

Decoder benchmarks live in `crates/fornax-decoder/benches/decoder.rs`. The benchmark
compares the DNG decoder and Rawler adapter over each `IDecoder` method and
over composite decode-plus-mosaic cases. `f32` is the default sample type;
`f64` and decoder subsets are opt-in. Preprocessor and demosaic benchmarks
live in `fornax-raw-pipeline`.

`decoder-tracker` validates the pointer-only raw.pixls.us submodule manifest and
runs manual, deterministically sharded decoder regressions against a committed
CSV baseline. RAW payloads are transient and never uploaded or committed.

See [the decoder-tracker agent guide](../../crates/decoder-tracker/AGENTS.md) for
corpus setup, run, baseline-update, and README rules.

`compare-demosaic` normalizes the active Bayer mosaic before passing it to
Fornax and pinned LibRaw, darktable, and RawTherapee adapters. Reference C/C++
code is build-time development tooling only and must not enter production
decoder dependencies.

See [the comparison agent guide](../../crates/compare-demosaic/AGENTS.md) for
baseline, waiver, artifact, and regeneration rules.

## References

- Decoder details: [docs/architecture/decoder.md](decoder.md)
- Decoder benchmark plan: [docs/decoder/benchmark.md](../decoder/benchmark.md)
- Decoder corpus plan: [docs/decoder/corpus.md](../decoder/corpus.md)
