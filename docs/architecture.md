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
| `fornax-core` | Shared image wrappers, process metadata, errors, and stage traits. | None |
| `fornax-tiff` | Generic TIFF tags, typed TIFF constants, values, directories, classic/BigTIFF parsing, and classic-TIFF writing. | None |
| `fornax-dng` | DNG-only tags, codecs, raw transforms, process-metadata conversion, and DNG writing over `fornax_tiff::Directory`. | `fornax-core`, `fornax-tiff` |
| `fornax-decoder` | Default pure-Rust DNG decoder adapter over `fornax-dng`. | `fornax-core`, `fornax-dng` |
| `fornax-decoder-rawler` | Alternative Rawler-backed decoder for supported 2x2 RGB Bayer RAW files. | `fornax-core`, `fornax-dng`, `fornax-tiff` |
| `fornax-decoder-dnc` | Converts with Adobe DNG Converter, then decodes through the Rawler adapter. | `fornax-core`, `fornax-decoder-rawler` |
| `fornax-preprocessor` | Black/white-level normalization of cropped Bayer mosaics. | `fornax-core` |
| `fornax-postprocessor` | Demosaic protocol, algorithms, and RGB reconstruction. | `fornax-core` |
| `fornax` | Public facade over decoders and pipeline stages. | Core, decoder crates, preprocessor, postprocessor |
| `fornax-cli` | Standalone batch converter for raw DNG, rendered TIFF/JPEG, and metadata JSON output. | Facade stages, DNG writer, Rawler adapter |
| `bench-decoder` | Decoder comparison benchmarks. | Core, decoder crates, devtool |
| `fornax-devtool` | Pinned fixture paths, synthetic data, and benchmark configuration. | `fornax-core` |
| `compare-demosaic` | Reference-versus-Fornax demosaic comparison harness. | Core, decoder, processors, devtool, pinned C/C++ references |
| `fornax-py` | PyO3/Maturin binding source, excluded from the Rust workspace while its API is stabilized. | External Python environment |

## Decoder contract

`fornax_core::protocol::IDecoder<I, T>` separates construction from access to
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

The default DNG implementation is `fornax_decoder::Decoder<T>`. The facade
exports it as `fornax::decoder::Decoder`, alongside `RawlerDecoder` and, on
Windows and macOS, the DNC decoder types. `fornax_core` contains no decoder
implementation.

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
`fornax-postprocessor` owns `IDemosaic<T>` and `LazyDemosaic<T>`.
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

`fornax_dng::DngWriter` regenerates uncompressed or lossless DNG data from
`RawMosaic` and a `fornax_tiff::Directory`. The lossless option writes tiled JPEG-92 for
U8/U16 samples and a lossless Deflate representation for U32/F32 samples.
Regeneration is not byte-identical to the source because the public model does
not retain the original container or auxiliary directories.

## Benchmarks and comparisons

Decoder benchmarks live in `bench-decoder/benches/decoder.rs`. The benchmark
compares the DNG decoder and Rawler adapter over each `IDecoder` method and
over composite decode-plus-mosaic cases. `f32` is the default sample type;
`f64` and decoder subsets are opt-in. Preprocessor and demosaic benchmarks
remain in their stage crates.

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
