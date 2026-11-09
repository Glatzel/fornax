# Current-to-target mapping

## Method and limits

`[TARGET]` This is a destination map, not an implemented move.

`[CURRENT]` File LOC below is non-blank line count from
`awk 'NF{n++} END{print n+0}' FILE`.

`[TARGET]` File-level destination is exact where one file has one owner.
`split` means the file contains more than one target concern; the named symbols
identify the proposed separation.

`[TARGET]` The totals apportion split files by the named symbol groups and are
therefore indicative rather than compiler-derived.

## Crate-level mapping

| `[CURRENT]` crate | `[TARGET]` destination | `[TARGET]` treatment |
| --- | --- | --- |
| `compare-demosaic` | `compare-demosaic` | Keep as a non-default tool; rewire types and commands |
| `decoder-tracker` | `decoder-tracker` | Keep; rewire decoder and metadata imports |
| `fornax` | `fornax-raw` | Retire the old facade package/name and preserve selected compatibility reexports |
| `fornax-arw` | `fornax-raw-formats` | Absorb ARW; split vendor codecs and generic TIFF helpers per OPEN 3 |
| `fornax-cli` | `fornax-cli` | Keep; depend on `fornax-raw` instead of `fornax` |
| `fornax-codec` | `fornax-codec` | Keep in base; replace raw output with neutral output and add features |
| `fornax-core` | split: `fornax-core` + `fornax-raw-types` + small `fornax-tiff`/codec basics | Neutral buffers/basics stay; raw domain moves |
| `fornax-decoder` | split: `fornax-raw` facade + `fornax-raw-formats` | Dispatch goes to facade; converter adapter remains a format tool |
| `fornax-devtool` | split: `fornax-devtool` + `fornax-raw-devtool` | Neutral helpers stay; raw fixtures move |
| `fornax-dng` | `fornax-dng` + `fornax-tiff::raster` | DNG policy stays; generic layout/helpers move |
| `fornax-postprocessor` | `fornax-raw-pipeline` | Merge under demosaic/color modules |
| `fornax-preprocessor` | `fornax-raw-pipeline` | Merge under normalize module |
| `fornax-tiff` | `fornax-tiff` + `fornax-dng` | Keep TIFF vocabulary; move DNG-specific constants to DNG |
| `fornax-py` | `fornax-py` | Keep excluded until API/buildability is resolved |

## File-level mapping

`[TARGET]` Every current `src/**/*.rs` file appears below.

### compare-demosaic

| `[CURRENT]` file and LOC | `[TARGET]` crate and module |
| --- | --- |
| `crates/compare-demosaic/src/artifacts.rs` (99) | `compare-demosaic::artifacts` |
| `crates/compare-demosaic/src/cases.rs` (492) | `compare-demosaic::cases`; update runner types |
| `crates/compare-demosaic/src/diff.rs` (523) | `compare-demosaic::diff` |
| `crates/compare-demosaic/src/lib.rs` (481) | `compare-demosaic::lib` |
| `crates/compare-demosaic/src/main.rs` (235) | `compare-demosaic::main` |
| `crates/compare-demosaic/src/metrics.rs` (561) | `compare-demosaic::metrics` |
| `crates/compare-demosaic/src/readme.rs` (850) | `compare-demosaic::readme` |
| `crates/compare-demosaic/src/reference.rs` (400) | `compare-demosaic::reference`; rewire decoder/preprocessor |
| `crates/compare-demosaic/src/report.rs` (626) | `compare-demosaic::report` |
| `crates/compare-demosaic/src/run.rs` (537) | `compare-demosaic::run`; rewire raw fixture paths |
| `crates/compare-demosaic/src/status.rs` (398) | `compare-demosaic::status` |
| `crates/compare-demosaic/src/waivers.rs` (34) | `compare-demosaic::waivers` |

### decoder-tracker

| `[CURRENT]` file and LOC | `[TARGET]` crate and module |
| --- | --- |
| `crates/decoder-tracker/src/csvio.rs` (126) | `decoder-tracker::csvio` |
| `crates/decoder-tracker/src/duckdb.rs` (159) | `decoder-tracker::duckdb` |
| `crates/decoder-tracker/src/lib.rs` (10) | `decoder-tracker::lib` |
| `crates/decoder-tracker/src/main.rs` (336) | `decoder-tracker::main` |
| `crates/decoder-tracker/src/manifest.rs` (225) | `decoder-tracker::manifest` |
| `crates/decoder-tracker/src/model.rs` (156) | `decoder-tracker::model` |
| `crates/decoder-tracker/src/runner.rs` (184) | `decoder-tracker::runner` |
| `crates/decoder-tracker/src/selection.rs` (116) | `decoder-tracker::selection` |
| `crates/decoder-tracker/src/stages.rs` (330) | `decoder-tracker::stages`; rewire raw types and facade |
| `crates/decoder-tracker/src/tests.rs` (641) | `decoder-tracker::tests`; update imports |

### fornax facade

| `[CURRENT]` file and LOC | `[TARGET]` crate and module |
| --- | --- |
| `crates/fornax/src/lib.rs` (9) | `fornax-raw::lib`; compose raw crates |
| `crates/fornax/src/postprocessor.rs` (3) | `fornax-raw` compatibility module or `fornax-raw-pipeline::compat` |
| `crates/fornax/src/preprocessor.rs` (1) | `fornax-raw` compatibility module or `fornax-raw-pipeline::compat` |

### fornax-arw

| `[CURRENT]` file and LOC | `[TARGET]` crate and module |
| --- | --- |
| `crates/fornax-arw/src/container.rs` (270) | **split**: Sony tags/layout/IFD selection to `fornax-raw-formats::arw::container`; `unsigned_scalar`, `unsigned_vec`, and generic strip/tile helpers to `fornax-tiff` |
| `crates/fornax-arw/src/decode.rs` (460) | **split/open**: format decode dispatch to `fornax-raw-formats::arw`; ARW1/ARW2/ARW6/LJPEG vendor functions to `fornax-codec` only if OPEN 3 chooses immediate movement |
| `crates/fornax-arw/src/lib.rs` (156) | `fornax-raw-formats::arw`; `ArwImage`; move `crop_and_cast` to shared TIFF/raw helper |
| `crates/fornax-arw/src/metadata.rs` (380) | `fornax-raw-formats::arw::metadata`; generic active-area/crop scalar helpers to `fornax-tiff`, Sony model exceptions stay vendor-specific |
| `crates/fornax-arw/src/sony.rs` (52) | `fornax-raw-formats::arw::sony` |
| `crates/fornax-arw/src/tests.rs` (213) | `fornax-raw-formats` ARW tests |

### fornax-cli

| `[CURRENT]` file and LOC | `[TARGET]` crate and module |
| --- | --- |
| `crates/fornax-cli/src/config.rs` (337) | `fornax-cli::config`; import `fornax-raw` and pipeline types |
| `crates/fornax-cli/src/convert.rs` (278) | `fornax-cli::convert`; use `fornax-raw::Decoder` |
| `crates/fornax-cli/src/discover.rs` (123) | `fornax-cli::discover`; use `RawFormat` from raw-types/facade |
| `crates/fornax-cli/src/lib.rs` (10) | `fornax-cli::lib` |
| `crates/fornax-cli/src/main.rs` (23) | `fornax-cli::main` |

### fornax-codec

| `[CURRENT]` file and LOC | `[TARGET]` crate and module |
| --- | --- |
| `crates/fornax-codec/src/deflate.rs` (84) | `fornax-codec::deflate`; feature-gated |
| `crates/fornax-codec/src/encode.rs` (109) | `fornax-codec::encode`; neutral buffer |
| `crates/fornax-codec/src/error.rs` (24) | `fornax-codec::error`; add `UnsupportedCompression` |
| `crates/fornax-codec/src/jpeg.rs` (93) | `fornax-codec::jpeg`; feature-gated |
| `crates/fornax-codec/src/jpegxl.rs` (160) | `fornax-codec::jpegxl`; feature-gated |
| `crates/fornax-codec/src/lib.rs` (131) | `fornax-codec::lib`; generic dispatch and typed vendor exports |
| `crates/fornax-codec/src/ljpeg.rs` (847) | `fornax-codec::ljpeg`; feature-gated |
| `crates/fornax-codec/src/predictor.rs` (299) | `fornax-codec::predictor` |
| `crates/fornax-codec/src/request.rs` (194) | **split**: `ByteOrder` to `fornax-core`; request/compression enums stay in `fornax-codec::request` |
| `crates/fornax-codec/src/samples.rs` (427) | `fornax-codec::samples`; return neutral buffer |
| `crates/fornax-codec/src/uncompressed.rs` (176) | `fornax-codec::uncompressed`; always compiled |
| `crates/fornax-codec/src/zstd.rs` (90) | `fornax-codec::zstd`; feature-gated |

### fornax-core

| `[CURRENT]` file and LOC | `[TARGET]` crate and module |
| --- | --- |
| `crates/fornax-core/src/bayer.rs` (156) | `fornax-raw-types::bayer` |
| `crates/fornax-core/src/error.rs` (63) | `fornax-raw-types::error`; only neutral error plumbing remains in `fornax-core::error` |
| `crates/fornax-core/src/format.rs` (35) | `fornax-raw-types::format` |
| `crates/fornax-core/src/lib.rs` (11) | **split**: reexports of neutral names stay in `fornax-core::lib`; raw names reexport from `fornax-raw-types::lib` |
| `crates/fornax-core/src/metadata.rs` (683) | `fornax-raw-types::metadata` and `fornax-raw-types::color` |
| `crates/fornax-core/src/primitive.rs` (73) | **split**: `Primitive`, `MonoImage`, `RgbImage`, and neutral buffer replacement to `fornax-core`; `RawMosaic`, `CroppedMosaic`, `NormalizedMosaic` to `fornax-raw-types::primitive` |
| `crates/fornax-core/src/protocol.rs` (48) | `fornax-raw-types::protocol` |
| `crates/fornax-core/src/utils.rs` (80) | `fornax-core::utils` |

### fornax-decoder

| `[CURRENT]` file and LOC | `[TARGET]` crate and module |
| --- | --- |
| `crates/fornax-decoder/src/dnc.rs` (354) | **open**: platform-gated external-converter adapter in `fornax-raw-formats` or raw-specific tooling; placement is not specified |
| `crates/fornax-decoder/src/lib.rs` (94) | **split**: format detection and `Decoder` dispatch to `fornax-raw::decoder`; format selection helpers may go to `fornax-raw-formats` |

### fornax-devtool

| `[CURRENT]` file and LOC | `[TARGET]` crate and module |
| --- | --- |
| `crates/fornax-devtool/src/bench.rs` (297) | **split**: `BenchParams`, sample-bits, size/thread parsing to `fornax-devtool::bench`; `DngBenchmarkInput`, `dng_benchmark_inputs`, `raw_mosaic_bytes` to `fornax-raw-devtool` |
| `crates/fornax-devtool/src/dirs.rs` (21) | **split**: `root_dir`, `temp_dir` stay neutral; `raw_file(RawFormat)` to `fornax-raw-devtool` |
| `crates/fornax-devtool/src/lib.rs` (78) | **split**: setup, synthetic mono buffer, save-image helpers stay neutral; raw reexports move to raw devtool |

### fornax-dng

| `[CURRENT]` file and LOC | `[TARGET]` crate and module |
| --- | --- |
| `crates/fornax-dng/src/dng_decoder/mod.rs` (918) | **split**: `DngImage`, DNG raw-IFD selection, DNG transform orchestration and public reader stay in `fornax-dng`; strip/tile plan and compression-ID mapping to `fornax-tiff::raster`; `unsigned_*` and `crop_and_cast` to shared TIFF/raw helpers |
| `crates/fornax-dng/src/dng_decoder/process_metadata.rs` (538) | **split**: DNG-to-`ProcessMetadata` mapping stays in `fornax-dng`; generic `active_area`, `default_crop`, scalar/list helpers move to `fornax-tiff` |
| `crates/fornax-dng/src/dng_decoder/transforms.rs` (381) | `fornax-dng::transforms`; opcode and linearization policy |
| `crates/fornax-dng/src/lib.rs` (11) | `fornax-dng::lib`; retain selected compatibility reexports |
| `crates/fornax-dng/src/metadata.rs` (249) | `fornax-dng::metadata`; `DngTag`, JSON metadata |
| `crates/fornax-dng/src/writer.rs` (662) | **split**: DNG output policy stays in `fornax-dng::writer`; generic chunk/framing helpers may use `fornax-tiff::raster` |

### fornax-postprocessor

| `[CURRENT]` file and LOC | `[TARGET]` crate and module |
| --- | --- |
| `crates/fornax-postprocessor/src/demosaic.rs` (365) | `fornax-raw-pipeline::demosaic` selector and lazy enum |
| `crates/fornax-postprocessor/src/demosaic/aahd.rs` (782) | `fornax-raw-pipeline::demosaic::aahd` |
| `crates/fornax-postprocessor/src/demosaic/ahd.rs` (417) | `fornax-raw-pipeline::demosaic::ahd` |
| `crates/fornax-postprocessor/src/demosaic/amaze.rs` (1575) | `fornax-raw-pipeline::demosaic::amaze` |
| `crates/fornax-postprocessor/src/demosaic/buffer.rs` (80) | `fornax-raw-pipeline::demosaic::buffer` |
| `crates/fornax-postprocessor/src/demosaic/dcb.rs` (1101) | `fornax-raw-pipeline::demosaic::dcb` |
| `crates/fornax-postprocessor/src/demosaic/dht.rs` (913) | `fornax-raw-pipeline::demosaic::dht` |
| `crates/fornax-postprocessor/src/demosaic/linear.rs` (416) | `fornax-raw-pipeline::demosaic::linear` |
| `crates/fornax-postprocessor/src/demosaic/lmmse.rs` (807) | `fornax-raw-pipeline::demosaic::lmmse` |
| `crates/fornax-postprocessor/src/demosaic/ppg.rs` (284) | `fornax-raw-pipeline::demosaic::ppg` |
| `crates/fornax-postprocessor/src/demosaic/protocol.rs` (81) | **split**: `IDemosaic`, `LazyDemosaic` to `fornax-raw-types::protocol`; lazy implementation details remain pipeline-internal |
| `crates/fornax-postprocessor/src/demosaic/rcd.rs` (524) | `fornax-raw-pipeline::demosaic::rcd` |
| `crates/fornax-postprocessor/src/demosaic/vng.rs` (503) | `fornax-raw-pipeline::demosaic::vng` |
| `crates/fornax-postprocessor/src/lib.rs` (410) | `fornax-raw-pipeline::color`; keep pipeline compatibility reexports |

### fornax-preprocessor

| `[CURRENT]` file and LOC | `[TARGET]` crate and module |
| --- | --- |
| `crates/fornax-preprocessor/src/lib.rs` (238) | `fornax-raw-pipeline::normalize`; compatibility `Preprocessor` name may remain |

### fornax-py

| `[CURRENT]` file and LOC | `[TARGET]` crate and module |
| --- | --- |
| `crates/fornax-py/src/lib.rs` (157) | `fornax-py::lib`; rewrite against resolved raw facade after Python work resumes |

### fornax-tiff

| `[CURRENT]` file and LOC | `[TARGET]` crate and module |
| --- | --- |
| `crates/fornax-tiff/src/constants.rs` (123) | **split**: TIFF 6.0/TIFF-EP constants stay in specification modules; DNG-specific values move to `fornax-dng`; raster compression mapping uses open numeric IDs |
| `crates/fornax-tiff/src/lib.rs` (53) | `fornax-tiff::lib`; optional `raster` module |
| `crates/fornax-tiff/src/parser.rs` (493) | `fornax-tiff::parser` |
| `crates/fornax-tiff/src/value.rs` (133) | `fornax-tiff::value`; keep `Tag` open and settle open `Value` representation |
| `crates/fornax-tiff/src/writer.rs` (284) | `fornax-tiff::writer` |

## LOC totals

`[CURRENT]` Total non-blank source LOC is 26,959 across the 88 current source
files, reproduced with
`find crates -path '*/src/*.rs' -type f -print0 | xargs -0 awk 'NF{n++} END{print n}'`.

`[TARGET]` The destination totals below include whole-file assignments and
explicit estimates for split files.

| `[TARGET]` target crate/workspace | LOC | Basis |
| --- | ---: | --- |
| `fornax-core` | ~103 | primitive neutral portion, utilities, byte-order basics |
| `fornax-codec` | 2,627 | current codec minus byte-order basics |
| `fornax-tiff` | ~1,356 | current TIFF, generic raster/layout additions, helper consolidation |
| `fornax-devtool` | ~330 | neutral devtool portion |
| `fornax-raw-types` | ~1,053 | current core domain/color/protocol content |
| `fornax-dng` | ~2,569 | current DNG minus generic raster/helper content, plus owned constants |
| `fornax-raw-formats` | ~1,805 | ARW implementation plus DNC adapter |
| `fornax-raw-pipeline` | 8,496 | preprocessor and postprocessor sources |
| `fornax-raw` | ~107 | current facade plus decoder dispatch |
| `compare-demosaic` | 5,236 | unchanged crate |
| `decoder-tracker` | 2,283 | unchanged crate |
| `fornax-cli` | 771 | unchanged crate |
| `fornax-py` | 157 | unchanged excluded crate |
| `fornax-raw-devtool` | ~66 | raw-specific devtool portion |
| **Total** | **26,959** | reconciled to current total |

`[TARGET]` Approximately 11,824 non-blank LOC cross a current crate boundary
under this attribution. Most of that is the mechanical merge of the two
pipeline crates into `fornax-raw-pipeline`.

| `[TARGET]` cross-boundary movement | Indicative LOC |
| --- | ---: |
| Pre/post merge into raw-pipeline | 8,496 |
| Core domain split into raw-types and byte-order basics | 1,060 |
| ARW implementation into raw-formats plus TIFF helpers | 1,531 |
| Decoder split into raw-formats and raw facade dispatch | 448 |
| DNG generic raster/helper portion into TIFF | 200 |
| Raw-specific devtool portion | 66 |
| Old facade into raw facade | 13 |
| DNG constants leaving TIFF | 10 |
| **Total** | **11,824** |

`[TARGET]` OPEN 3 is excluded from this total. Immediate movement of Sony
codec functions would shift up to the vendor-codec portion of
`crates/fornax-arw/src/decode.rs` from `fornax-raw-formats` to
`fornax-codec`.

## API path breakage

`[CURRENT]` Usage counts below are workspace file and token counts reproducible
with this loop over the names in the table:

```text
for t in BayerChannel BayerPattern RawFormat RawMosaic CroppedMosaic NormalizedMosaic Primitive MonoImage RgbImage RawValidationError MetadataValidationError DecodeError PreprocessError PostProcessError DemosaicError Matrix3x3 Array3 ProcessMetadata Levels LightSource IlluminantData RawColorMeta RawProfile WhiteSource WhiteSpec ColorTransform ColorCalibration xy_to_cct cct_to_xy IDecoder IPreprocessor IPostProcessMetadata IPostProcessor IDemosaic LazyDemosaic Preprocessor Demosaic LmmseMode OutputColorSpace PostProcessor ArwImage Decoder CompressionMethod PhotometricInterpretation DngTag DngCompression DngVersion DngWriter Directory Value Tag DecodeRequest EncodeRequest EncodedChunks CodecError; do
  printf '%s files=' "$t"; rg -l "\b$t\b" crates -g '*.rs' | wc -l
  printf '%s uses=' "$t"; rg -o "\b$t\b" crates -g '*.rs' | wc -l
done
```

| Current item or family | Target path | Workspace files using name | Facade-reexported today? |
| --- | --- | ---: | --- |
| `BayerChannel` | `fornax_raw_types::BayerChannel` | 17 | yes |
| `BayerPattern` | `fornax_raw_types::BayerPattern` | 31 | yes |
| `RawFormat` | `fornax_raw_types::RawFormat` | 11 | yes |
| `RawMosaic` | `fornax_raw_types::RawMosaic` | 27 | yes |
| `CroppedMosaic` | `fornax_raw_types::CroppedMosaic` | 11 | yes |
| `NormalizedMosaic` | `fornax_raw_types::NormalizedMosaic` | 5 | yes |
| `Primitive`, `MonoImage`, `RgbImage` | remain under `fornax_core`; exact neutral names follow OPEN 1 | 32, 42, 13 | yes |
| `RawValidationError` | `fornax_raw_types::error::RawValidationError` | 2 | via `error` |
| `MetadataValidationError` | `fornax_raw_types::error::MetadataValidationError` | 1 | via `error` |
| `DecodeError` | `fornax_raw_types::error::DecodeError` | 18 | via `error` |
| `PreprocessError` | `fornax_raw_types::error::PreprocessError` | 6 | via `error` |
| `PostProcessError` | `fornax_raw_types::error::PostProcessError` | 5 | via `error` |
| `DemosaicError` | `fornax_raw_types::error::DemosaicError` | 14 | via `error` |
| `Matrix3x3`, `Array3` | `fornax_raw_types::color` | 3, 2 | via `metadata` |
| `ProcessMetadata`, `Levels` | `fornax_raw_types::metadata` | 10, 8 | via `metadata` |
| `LightSource`, `IlluminantData`, `RawColorMeta`, `RawProfile` | `fornax_raw_types::color` | 3, 1, 4, 3 | via `metadata` |
| `WhiteSource`, `WhiteSpec`, `ColorTransform`, `ColorCalibration` | `fornax_raw_types::color` | 2, 1, 2, 5 | via `metadata` |
| `xy_to_cct`, `cct_to_xy` | `fornax_raw_types::color` | 1, 1 | via `metadata` |
| `IDecoder` | `fornax_raw_types::protocol::IDecoder` | 10 | yes |
| `IPreprocessor` | `fornax_raw_types::protocol::IPreprocessor` | 6 | yes |
| `IPostProcessMetadata` | `fornax_raw_types::protocol::IPostProcessMetadata` | 4 | yes |
| `IPostProcessor` | `fornax_raw_types::protocol::IPostProcessor` | 4 | yes |
| `IDemosaic`, `LazyDemosaic` | `fornax_raw_types::protocol` | 8, 18 | postprocessor module |
| `Preprocessor` | `fornax_raw_pipeline::Preprocessor` | 5 | `fornax::preprocessor` |
| `Demosaic`, `LmmseMode` | `fornax_raw_pipeline::demosaic` | 10, 3 | `fornax::postprocessor` |
| `OutputColorSpace`, `PostProcessor` | `fornax_raw_pipeline::color` | 3, 3 | `fornax::postprocessor` |
| `ArwImage` | `fornax_raw_formats::ArwImage` | 3 | `fornax::arw::ArwImage` |
| `Decoder` | `fornax_raw::Decoder` | 8 | `fornax::Decoder` |
| `CompressionMethod` | split: TIFF raster request mapping plus DNG-specific IDs | 3 | no direct facade name |
| `PhotometricInterpretation` | split: TIFF/EP values versus DNG-owned values | 8 | no direct facade name |
| `DngTag`, `DngCompression`, `DngVersion`, `DngWriter` | unchanged `fornax_dng` names | 10, 6, 7, 6 | `fornax::dng` |
| `Directory`, `Value`, `Tag` | unchanged `fornax_tiff` definitions | 20, 20, 19 | `dng` reexports `Directory`/`Value`, not `Tag` |
| `DecodeRequest`, `EncodeRequest`, `EncodedChunks`, `CodecError` | unchanged `fornax_codec` names; buffer fields change | 11, 8, 7, 14 | no |
| `fornax::{preprocessor, postprocessor, arw, dng}` and root reexports | `fornax_raw` modules/reexports | 5 files / 20 `fornax::` paths | package-level facade rename |

`[TARGET]` Forty-one primary root-reachable type/trait/item names change crate path in the table
above. Inherent methods, public fields, enum variants, trait methods, and
facade reexports travel with those names and therefore break at the same time.

`[TARGET]` Compatibility policy is not decided. Reexports can preserve some
paths, but package-level moves such as `fornax` to `fornax-raw` cannot be
hidden without a compatibility package.

`[TARGET]` `fornax-cli` imports the facade at
`crates/fornax-cli/src/convert.rs:5`; `decoder-tracker` imports internal types
at `crates/decoder-tracker/src/stages.rs:5`; `compare-demosaic` imports internal
types at `crates/compare-demosaic/src/reference.rs:4`; all three must change in
the step that moves their dependencies.

## Type moves

`[CURRENT]` Importing crates in the final column are reproduced with the same
name loop used for API usage counts, replacing the count with
`rg -l "\b$t\b" crates/*/src crates/*/tests crates/*/benches -g '*.rs' | cut -d/ -f2 | sort -u`.

| Type or protocol | `[CURRENT]` definition | `[TARGET]` owner | `[CURRENT]` importing crates |
| --- | --- | --- | --- |
| `RawMosaic` | `crates/fornax-core/src/primitive.rs:24` | `fornax-raw-types`, reconciled with OPEN 1 buffer | arw, cli, codec, core, decoder, devtool, dng, tracker, facade |
| `MonoImage` | `crates/fornax-core/src/primitive.rs:19` | `fornax-core` neutral image vocabulary | arw, cli, codec, compare, core, decoder, devtool, dng, post, pre, tracker, facade |
| `CroppedMosaic` | `crates/fornax-core/src/primitive.rs:46` | `fornax-raw-types` | arw, compare, core, decoder, dng, pre, facade |
| `NormalizedMosaic` | `crates/fornax-core/src/primitive.rs:68` | `fornax-raw-types` | core, pre, facade |
| `BayerPattern` | `crates/fornax-core/src/bayer.rs:31` | `fornax-raw-types` | compare, core, dng, post, tracker, facade |
| `Levels` | `crates/fornax-core/src/metadata.rs:60` | `fornax-raw-types` | core, dng, post, pre, tracker |
| `ProcessMetadata` | `crates/fornax-core/src/metadata.rs:27` | `fornax-raw-types` | arw, core, decoder, dng, post, tracker |
| `RawFormat` | `crates/fornax-core/src/format.rs:16` | `fornax-raw-types` | cli, compare, core, decoder, devtool, dng, facade |
| `IDecoder` | `crates/fornax-core/src/protocol.rs:8` | `fornax-raw-types` | arw, cli-through-facade, compare, core, decoder, dng, tracker |
| `IPreprocessor` | `crates/fornax-core/src/protocol.rs:19` | `fornax-raw-types` | compare, core, pre, facade, cli-through-facade |
| `IPostProcessMetadata`, `IPostProcessor` | `crates/fornax-core/src/protocol.rs:29`, `crates/fornax-core/src/protocol.rs:43` | `fornax-raw-types` | core, post, facade, cli-through-facade |
| `IDemosaic`, `LazyDemosaic` | `crates/fornax-postprocessor/src/demosaic/protocol.rs:19`, `crates/fornax-postprocessor/src/demosaic/protocol.rs:81` | `fornax-raw-types` | compare, post, tests |
| `DecodeRequest` | `crates/fornax-codec/src/request.rs:130` | `fornax-codec`; output becomes neutral | codec, dng |
| `Directory` | `crates/fornax-tiff/src/lib.rs:16` | `fornax-tiff` | arw, cli, decoder, dng, tiff, tracker |
| `Value` | `crates/fornax-tiff/src/value.rs:23` | `fornax-tiff`, open representation target | arw, cli, dng, tiff, tracker |
| `Tag` | `crates/fornax-tiff/src/value.rs:5` | `fornax-tiff`, open identifier target | arw, cli, dng, tiff, tracker |
| `CompressionMethod` | `crates/fornax-tiff/src/constants.rs:85` | compression mapping split between TIFF raster and DNG-owned constants | dng, tiff |

## Constant audit

`[CURRENT]` The current TIFF constant enums are at
`crates/fornax-tiff/src/constants.rs:13`,
`crates/fornax-tiff/src/constants.rs:85`, and
`crates/fornax-tiff/src/constants.rs:108`.

| Current constant | Origin evidence | `[TARGET]` disposition |
| --- | --- | --- |
| `CompressionMethod::Uncompressed = 1` | TIFF 6.0 compression 1, represented at `crates/fornax-tiff/src/constants.rs:87` | Base TIFF or shared codec request value |
| `CompressionMethod::LosslessJpeg = 7` | TIFF 6.0 compression 7, represented at `crates/fornax-tiff/src/constants.rs:89` | Base TIFF/raster mapping |
| `CompressionMethod::Deflate = 8` | Fornax DNG notes list it at `docs/references/dng-compression.md:16`; no normative origin is cited in code | Keep as raster mapping; exact spec origin is an evidence gap |
| `CompressionMethod::LossyJpeg = 34892` | Fornax DNG notes list it at `docs/references/dng-compression.md:17` | DNG-owned vocabulary or open numeric raster mapping |
| `CompressionMethod::Zstd = 50000` | `docs/references/dng-compression.md:24` says it is a libtiff/GDAL de-facto assignment, not Adobe-registered DNG | Codec/raster extension, not a DNG-owned constant |
| `CompressionMethod::JpegXlLegacy = 50002` | `docs/references/dng-compression.md:21` calls it the legacy TIFF JPEG XL identifier | TIFF extension/raster mapping or codec-owned identifier |
| `CompressionMethod::JpegXl = 52546` | `docs/references/dng-compression.md:21` says DNG 1.7 defines it | Move DNG definition out of `fornax-tiff`; raster uses numeric/open mapping |
| `PhotometricInterpretation::Cfa = 32803` | Fornax DNG reference uses CFA at `docs/references/dng-lossless-jpeg.md:26`; TIFF/EP origin is not stated in this repository | Keep if TIFF/EP ownership is confirmed; otherwise DNG-owned |
| `PhotometricInterpretation::LinearRaw = 34892` | Used as DNG raw photometric value at `crates/fornax-dng/src/dng_decoder/mod.rs:55` | Move DNG definition out of `fornax-tiff`; raster uses numeric/open mapping |
| `TiffTag::CfaRepeatPatternDim`, `TiffTag::CfaPattern` | Represented as TIFF/EP fields at `crates/fornax-tiff/src/constants.rs:56` and `crates/fornax-tiff/src/constants.rs:58` | Stay in TIFF/EP specification module |

`[TARGET]` The exact origin of `Deflate`, `LossyJpeg`, and `Cfa` is not
settled by repository evidence; it is listed in
[06-open-decisions.md](06-open-decisions.md).

## Duplicated helpers

| Helper family | DNG occurrence | ARW occurrence | `[TARGET]` destination |
| --- | --- | --- | --- |
| `unsigned_scalar` | `crates/fornax-dng/src/dng_decoder/mod.rs:340` | `crates/fornax-arw/src/container.rs:236` | `fornax-tiff` value accessors |
| DNG-local `unsigned_scalar` | `crates/fornax-dng/src/dng_decoder/process_metadata.rs:405` | overlapped with ARW scalar conversion | `fornax-tiff` conversion/accessor API |
| `unsigned_vec` | `crates/fornax-dng/src/dng_decoder/mod.rs:351` | `crates/fornax-arw/src/container.rs:247` | `fornax-tiff` value accessors |
| `active_area` | `crates/fornax-dng/src/dng_decoder/process_metadata.rs:157` | `crates/fornax-arw/src/metadata.rs:189` | Shared rectangle helper plus tag-specific callers |
| `default_crop` | `crates/fornax-dng/src/dng_decoder/process_metadata.rs:185` | `crates/fornax-arw/src/metadata.rs:210` | Shared rectangle helper plus vendor defaults |
| `crop_and_cast` | `crates/fornax-dng/src/dng_decoder/mod.rs:130` | `crates/fornax-arw/src/lib.rs:158` | Shared raw/TIFF image helper |

## Vendor codecs in format crates

| `[CURRENT]` codec family | Implementation | External dependency | Used by |
| --- | --- | --- | --- |
| Sony uncompressed 12/14/16-bit | `crates/fornax-arw/src/decode.rs:22` | none beyond local logic | `fornax-arw` |
| Sony ARW1 | `crates/fornax-arw/src/decode.rs:71` | none beyond local logic | `fornax-arw` |
| Sony ARW2 | `crates/fornax-arw/src/decode.rs:103` | `flashraw-codecs` curve support via `crates/fornax-arw/src/metadata.rs:325` | `fornax-arw` |
| Sony ARW6 | `crates/fornax-arw/src/decode.rs:232` | `flashraw-codecs` at `crates/fornax-arw/src/decode.rs:236` | `fornax-arw` |
| Sony tiled lossless JPEG | `crates/fornax-arw/src/decode.rs:241` | `flashraw-codecs` at `crates/fornax-arw/src/decode.rs:1` | `fornax-arw` test encoder also uses it at `crates/fornax-arw/src/decode.rs:469` |
| DNG opcode processing | `crates/fornax-dng/src/dng_decoder/transforms.rs:1` | `flashraw-codecs::dng_opcodes` | `fornax-dng` |

`[TARGET]` The Sony codec families move to `fornax-codec` immediately only if
OPEN 3 says so; otherwise they remain typed vendor functions in
`fornax-raw-formats` until a second format justifies shared ownership.
