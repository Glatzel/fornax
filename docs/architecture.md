# Architecture

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
| `fornax-tiff` | Generic TIFF tags, typed TIFF constants, values, directories, raster strip/tile planning, classic/BigTIFF parsing, and classic-TIFF writing. | `fornax-codec` |
| `fornax-codec` | Format-neutral raw sample decoding, chunk assembly, predictors, and supported codec encoding. | None |
| `fornax-dng` | DNG-only tags, raw transforms, process-metadata conversion, and DNG writing over `fornax_tiff::Directory`. | `fornax-raw-core`, `fornax-tiff`, `fornax-codec` |
| `fornax-arw` | Native Sony ARW/NRW TIFF, private metadata, CFA sample decoding, and DNG-compatible metadata mapping. | `fornax-raw-core`, `fornax-tiff`, `fornax-dng`, `flashraw-codecs` |
| `fornax-decoder` | Default pure-Rust DNG and Sony ARW/NRW decoder facade over the format crates. | `fornax-raw-core`, `fornax-dng`, `fornax-arw` |
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
[reserved decision](proposal/steps-1-6-review-summary.md#11-decisions-and-gaps).

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
as `fornax_raw::Decoder`; it dispatches DNG containers to `fornax-dng` and ARW/NRW
containers to `fornax-arw`. `fornax_raw_core` contains no decoder implementation.

## Codec API

`fornax_codec::DecodeCompression` and
`fornax_codec::EncodeCompression` select algorithms inside `DecodeRequest` and
`EncodeRequest`, respectively. The free `fornax_codec::decode` and
`fornax_codec::encode` functions dispatch directly to the selected
implementation and exchange a single-band `fornax_codec::SampleBuffer`.
`SampleBuffer` owns U8, U16, U32, or F32 samples in an
`image::ImageBuffer<Luma<T>, Vec<T>>`; `Luma` denotes one scalar channel rather
than necessarily luminance. Zstandard decoding has no level because the level
is stored in the stream; `EncodeCompression::Zstd { level }` configures
encoding.

`fornax-tiff::CompressionMethod` remains the typed TIFF field value.
`fornax_tiff::RasterLayout` owns strip/tile geometry and chunk ordering, while
`fornax_tiff::RasterRequest` accepts an already-resolved codec compression.
`resolve_tiff_compression` supplies TIFF defaults `1`, `7`, and `8`.
These raster integration APIs are behind `fornax-tiff`'s non-default `raster`
feature. The feature enables its optional `fornax-codec` dependency; TIFF-only
consumers can use the parser, values, tags, and writer without codec, image, or
Rayon dependencies. `fornax-dng` enables `raster` on its TIFF dependency.
`fornax_dng::constants` owns DNG compression and photometric values, including
the compatibility IDs interpreted only by the DNG format layer. `fornax-dng`
resolves those values before falling back to the TIFF defaults. `fornax-codec`
therefore does not depend on TIFF, DNG, or raw workflow types. `fornax-dng`
adapts codec `SampleBuffer` values to and from `fornax_raw_core::RawMosaic` at its
decode and write boundaries, keeping `RawMosaic` ownership unchanged.

Codec availability is mandatory in the decoder path. Zstandard and all future
codecs are compiled unconditionally without per-codec features or runtime
fallback errors. The `fornax-raw` facade exposes a `zstd` feature that forwards
to `fornax-decoder/zstd`; it is retained by maintainer decision.

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

Postprocessing can emit camera-native RGB or convert scene-linear samples to
sRGB, Adobe RGB, Wide Gamut RGB, ProPhoto RGB, XYZ, ACES2065-1, ACEScg, DCI-P3,
or Rec. 2020. The default remains camera-native RGB.

`default_crop` is a recommendation. Stage methods do not apply it automatically.

## DNG support

The default decoder reads classic TIFF and BigTIFF directories, selects the
full-resolution CFA or LinearRaw image, preserves typed root and raw-IFD
metadata, and decodes uncompressed, lossless JPEG-92, Deflate, lossy JPEG, and
JPEG XL raw chunks. Independent strips and tiles are decoded with Rayon. Raw
storage transforms, linearization, and `OpcodeList1` are applied before
returning the mosaic. Its process-metadata conversion accepts only conventional
2x2 RGB Bayer layouts.

Compression `52546` is the DNG 1.7 JPEG XL identifier and compression `50002`
is the legacy TIFF JPEG XL identifier. Both map to the single-channel decoder.
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
parses and decrypts Sony `DNGPrivateData`, and maps geometry, CFA, black/white
levels, white balance, and crop data into the shared DNG metadata model. It
supports uncompressed 12-bit packed and 14/16-bit word samples, ARW1, ARW2,
ARW6, and single-component tiled LJPEG. Four-component Sony LJPEG tiles are
unpacked into two Bayer output rows per JPEG row.

Reduced-resolution YCbCr/`LinearRaw` images, multi-sample ARQ files, unknown
compression families, and non-Bayer CFAs return typed unsupported errors.
Camera-specific color matrices are not synthesized when the source container
does not provide them.

## Benchmarks and comparisons

Decoder benchmarks live in `crates/fornax-decoder/benches/decoder.rs`. The benchmark
compares the DNG decoder and Rawler adapter over each `IDecoder` method and
over composite decode-plus-mosaic cases. `f32` is the default sample type;
`f64` and decoder subsets are opt-in. Preprocessor and demosaic benchmarks
live in `fornax-raw-pipeline`.

`decoder-tracker` validates the pointer-only raw.pixls.us submodule manifest and
runs manual, deterministically sharded decoder regressions against a committed
CSV baseline. RAW payloads are transient and never uploaded or committed.

See [the decoder-tracker agent guide](../crates/decoder-tracker/AGENTS.md) for
corpus setup, run, baseline-update, and README rules.

`compare-demosaic` normalizes the active Bayer mosaic before passing it to
Fornax and pinned LibRaw, darktable, and RawTherapee adapters. Reference C/C++
code is build-time development tooling only and must not enter production
decoder dependencies.

See [the comparison agent guide](../crates/compare-demosaic/AGENTS.md) for
baseline, waiver, artifact, and regeneration rules.

## References

- Decoder details: [docs/decoder/architecture.md](decoder/architecture.md)
- Decoder benchmark plan: [docs/decoder/benchmark.md](decoder/benchmark.md)
- Decoder corpus plan: [docs/decoder/corpus.md](decoder/corpus.md)
