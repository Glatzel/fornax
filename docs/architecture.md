# Architecture

## Purpose

Fornax is a Rust RAW-image processing workspace with separate decode,
preprocess, demosaic, and post-process stages. The production decoder is the
pure-Rust classic-TIFF DNG implementation in `fornax-core`. The public facade
and stage crates do not depend on Rawler, LibRaw, Adobe DNG Converter, or a
decoder C/C++ binding.

The workspace uses Rust edition 2024. Pipeline images use `image` buffer aliases
through `fornax-core`: `MonoImage<T>`, `MosaicData`, `RawImage<T>`, and
`RgbImage<T>`.

## Crate map

| Crate | Role | Internal dependencies |
| --- | --- | --- |
| `fornax-core` | Pipeline types and traits, DNG decoding, typed metadata, and DNG writing. | None |
| `fornax-preprocessor` | Normalizes active Bayer mosaics with black and white levels. | `fornax-core` |
| `fornax-postprocessor` | Demosaic protocol and algorithms from Bayer mosaic to RGB. | `fornax-core` |
| `fornax` | Public facade over the production pipeline. | `fornax-core`, preprocessor, postprocessor |
| `fornax-devtool` | Pinned fixture paths and synthetic development helpers. | `fornax-core` |
| `compare-demosaic` | Reference-versus-Fornax comparison harness for demosaic algorithms. | Core, processors, devtool, pinned C/C++ references |
| `fornax-py` | PyO3/Maturin Python binding source. | Excluded from the Rust workspace while its API is stabilized |

Criterion benchmarks live with the crates they measure:
`fornax-core/benches/decoder.rs`, `fornax-preprocessor/benches/`, and
`fornax-postprocessor/benches/`.

## Decode

`fornax_core::protocol::IDecoder<I, T>` defines the shared decoder boundary:

```rust
fn decode(&self, input: I) -> Result<(MosaicData, Metadata<T>), DecodeError>;
fn decode_active(&self, input: I)
    -> Result<(DecodedRaw<T>, Metadata<T>), DecodeError>;
```

`fornax_core::Decoder` implements `IDecoder<&Path, T>` for floating-point
pipeline output. It currently accepts classic-TIFF DNG files with uncompressed
or lossless JPEG-92 strips and tiles. Decoding selects the full-resolution CFA
IFD, preserves root and raw-IFD values in `DngMetadata`, validates geometry and
levels, and returns the full `MosaicData`.

`decode_active` crops to `Metadata::active_area`, shifts the Bayer phase by the
active-area origin, and returns `DecodedRaw<T>`. The current metadata conversion
accepts only conventional RGB 2x2 CFA arrays; other layouts return a typed
unsupported error.

`Metadata<T>` carries:

- active and default-crop rectangles;
- black-level pattern, row/column deltas, and white level;
- the full-frame Bayer pattern;
- optional green-normalized white balance;
- the original typed `DngMetadata`.

## Preprocess

`DecodedRaw<T>::process` delegates to `IPreprocessor<T>`. The production
`Preprocessor` subtracts the repeating black-level pattern and optional
row/column deltas, normalizes by the remaining white-level range, and preserves
a single-channel mosaic.

The decode-to-preprocess boundary is:

```text
DecodedRaw<T> -> RawImage<T>
```

## Demosaic and post-process

`RawImage<T>::process` delegates to `IPostProcessor<T, M>`. The production
postprocessor calls an `IDemosaic<T>` implementation with the normalized mosaic
and Bayer pattern. `LazyDemosaic<T>` produces RGB rows and is collected into an
`RgbImage<T>`.

The preprocess-to-demosaic boundary is:

```text
RawImage<T> -> RgbImage<T>
```

The postprocessor supports Linear, PPG, RCD, AHD, VNG, VNG4, DCB, DHT, AAHD,
AMaZE, and LMMSE. `PostProcessor::default()` selects Linear and applies
green-normalized white balance when metadata provides it.

`Metadata::default_crop` is carried as a recommendation; stage methods do not
apply it automatically.

## DNG writing

`fornax_core::writer::DngWriter` currently regenerates an uncompressed DNG from
`MosaicData` and `DngMetadata`. Lossless-JPEG output remains a typed
unsupported operation because the current decoded-data API does not retain the
source container required for byte-identical export.

## Supported scope

The active production decoder supports the implemented classic-TIFF DNG path.
The active pipeline handoff and demosaic target are conventional 2x2 RGB Bayer
layouts: RGGB, BGGR, GRBG, and GBRG. X-Trans, four-color, monochrome, BigTIFF,
and unsupported DNG compression modes are not claimed as supported.

Binary RAW fixtures and generated DNG/TIFF output remain outside Git. The
accepted DNG fixture is supplied through the `external/raw-images` submodule.

## Comparison tooling

`compare-demosaic` passes the normalized active Bayer mosaic into Fornax and
pinned LibRaw, darktable, and RawTherapee reference adapters at the same stage.
The committed `canonical.json` baseline uses
`colorchart-5D2-6000K.dng`. Reference C/C++ code is build-time development
tooling only and must not enter the production decoder dependency tree.

See [the comparison agent guide](../crates/compare-demosaic/AGENTS.md) for
baseline, waiver, and artifact rules.
