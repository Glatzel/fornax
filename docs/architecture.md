# Architecture

## Purpose

Fornax is a Rust raw-image processing workspace for decoding camera RAW files,
normalizing the active sensor mosaic, and reconstructing RGB output. It exposes
typed stage boundaries so users can select a built-in backend or implement a
decoder, preprocessor, or postprocessor. The output stage types are
`image::ImageBuffer` aliases: `MonoImage<T>` for single-channel mosaics and
`RgbImage<T>` for three-channel output.

The workspace uses Rust edition 2024. The `fornax` crate is the public facade;
the other library crates provide shared types, concrete stages, decoder
backends, low-level bindings, development helpers, benchmarks, and comparison
tooling.

## Crate map

| Crate | Role | Internal dependencies |
| --- | --- | --- |
| `fornax-core` | Shared image aliases, Bayer types, metadata, errors, decoded-image wrappers, and stage traits. | None |
| `fornax-decoder-rawler` | Decodes conventional 2x2 RGB Bayer RAW files with Rawler. | `fornax-core` |
| `fornax-decoder-dnc` | Converts RAW with Adobe DNG Converter, then decodes through Rawler; available on Windows and macOS. | `fornax-core`, `fornax-decoder-rawler` |
| `libraw-sys` | Low-level Rust FFI for the LibRaw C API. | None |
| `libraw` | Safe LibRaw data types, decoder adapter, and processed-image interface. | `fornax-core`, `libraw-sys` |
| `fornax-preprocessor` | Normalizes decoded active-area mosaics using black and white levels. | `fornax-core` |
| `fornax-postprocessor` | Implements demosaic algorithms and converts mosaics to RGB. | `fornax-core` |
| `fornax` | Public facade and feature-gated decoder exports. | `fornax-core`, both Rust decoders, `libraw`, both processors |
| `fornax-py` | PyO3/Maturin Python binding source. | `fornax`, `libraw`, Python-facing crates |
| `fornax-devtool` | Shared fixtures, synthetic images, tracing setup, and image helpers. | `fornax-core` |
| `bench-decoder`, `bench-preprocessor`, `bench-demosaic` | Criterion benchmark targets for their named stages. | Corresponding stage crates and `fornax-devtool` |
| `compare-demosaic` | Separate reference-versus-Fornax demosaic harness and CLI. | Core, processors, Rawler decoder, devtool, `libraw-sys` build dependency |

`Cargo.toml` includes every crate under `crates/` in the workspace except
`fornax-py`, which is temporarily excluded while its public API is stabilized.

## Pipeline

The pipeline boundaries are expressed by distinct types and traits in
`fornax-core`, not only by convention.

### Decode

`IDecoder<I, T>` in `crates/fornax-core/src/protocol.rs` accepts an input such
as a path or byte slice and returns a decoded mosaic plus `Metadata<T>`.
`decode_full` returns `DecodedRawFull<T>`; `decode_active` returns
`DecodedRawActive<T>` after cropping to `Metadata::active_area`.

`Metadata<T>` carries the active area, recommended default crop, black and
white levels, Bayer pattern, and optional white balance. Decoder-specific code
converts backend metadata into this common form. The Rawler decoder rejects
non-CFA payloads and CFA layouts other than 2x2 RGB Bayer.

### Preprocess

`DecodedRawActive<T>::process` delegates to `IPreprocessor<T>` with
`Metadata::levels`. The default `Preprocessor` subtracts the repeating black
level pattern and optional row/column deltas, divides by the remaining
white-level range, and clamps negative values to zero while retaining a
single-channel mosaic. It returns `RawImage<T>`.

The boundary between decode and preprocess is therefore
`DecodedRawActive<T> -> RawImage<T>`. The preprocessor implementation lives in
`crates/fornax-preprocessor/src/lib.rs`.

### Demosaic and post-process

`RawImage<T>::process` delegates to `IPostProcessor<T, M>` with metadata.
`PostProcessor` calls an `IDemosaic<T>` implementation with the mosaic and
`Metadata::bayer_pattern`. `IDemosaic` returns a `LazyDemosaic<T>`, which
produces RGB rows and then an `RgbImage<T>`.

The postprocessor supports Linear, PPG, RCD, AHD, VNG, DCB, DHT, AAHD, AMaZE,
and LMMSE reconstruction. `PostProcessor::default()` selects Linear
demosaicing and applies green-normalized white balance when metadata provides
it. The boundary between preprocessing and demosaicing is
`RawImage<T> -> RgbImage<T>`; implementations live in
`crates/fornax-postprocessor/src/`.

`Metadata::default_crop` is carried through the stages as a recommendation.
The current stage methods do not apply it automatically.

## Supported scope

The active demosaic target is conventional 2x2 RGB Bayer input using one of
the four `BayerPattern` values: RGGB, BGGR, GRBG, or GBRG. X-Trans, four-color,
monochrome, and other CFA layouts are not supported and must not be claimed as
supported.

## Interfaces

- Rust callers use the stage traits from `fornax-core` or the re-exports in the
  `fornax` facade. `crates/fornax/examples/pipeline.rs` demonstrates
  decode-active, preprocess, and demosaic calls.
- `libraw-sys` exposes unsafe LibRaw FFI declarations, and `libraw` provides
  safe wrappers plus `LibrawDecoder`. This is a binding to LibRaw, not a
  general-purpose Fornax C API.
- `crates/fornax-py` contains a PyO3/Maturin binding targeting Python 3.13.
  It is excluded from the Rust workspace, so its public Python API is not
  presented here as stable.
- `compare-demosaic` provides the repository's comparison CLI. The benchmark
  crates are development interfaces rather than application APIs.

## Comparison tooling

`compare-demosaic` compares Fornax demosaic output with LibRaw and darktable
at the same stage: a normalized active Bayer mosaic enters each
implementation and reconstructed RGB leaves it. Its committed baseline is the
source of truth for reviewed results. Algorithm-independent comparison rules,
including waivers, CLI behavior, source pinning, and CI, are documented in the
[comparison agent guide](../crates/compare-demosaic/AGENTS.md).
