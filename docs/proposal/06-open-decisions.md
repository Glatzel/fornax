# Open decisions and evidence gaps

`[TARGET]` These decisions are intentionally unresolved. The proposal records
options, evidence, and blocked work without selecting an answer.

## OPEN 1: neutral buffer shape

`[TARGET]` Option A: one enum over sample types (`u8`, `u16`, `u32`, `f32`)
with a band count.

`[TARGET]` Option B: a generic `Raster<T>` carrying width, height, and band
count.

`[CURRENT]` Today `RawMosaic` is an enum of single-channel `MonoImage`
buffers at `crates/fornax-core/src/primitive.rs:24`.

`[CURRENT]` Codec output always builds a mono image:
`crates/fornax-codec/src/samples.rs:452`.

`[CURRENT]` Demosaic accepts `&MonoImage<T>` and a Bayer pattern at
`crates/fornax-postprocessor/src/demosaic/protocol.rs:81`.

`[CURRENT]` The DNG decoder rejects `SamplesPerPixel != 1` at
`crates/fornax-dng/src/dng_decoder/mod.rs:208`.

`[TARGET]` Option A minimizes changes for existing raw/mono callers but makes
multi-band future rasters less type-safe.

`[TARGET]` Option B gives stronger sample typing across codec and geography
use cases but risks generic churn through every decoder and pipeline call.

`[TARGET]` This decision blocks the final public shape of `fornax-core`,
`DecodeRequest`/encode output, `RawMosaic`, and all buffer adapters.

## OPEN 2: fold `fornax-core` into `fornax-codec`

`[TARGET]` Option A: keep a separate neutral `fornax-core`.

`[TARGET]` Option B: later fold core into codec if core remains tiny.

`[CURRENT]` Current core owns 1,149 LOC, reproduced by the LOC command in
[README.md](README.md), but 1,053 LOC are identified as raw
domain/color/protocol content that moves to `fornax-raw-types`; only utilities,
primitive bounds, and buffer aliases are visibly neutral at
`crates/fornax-core/src/utils.rs:4` and
`crates/fornax-core/src/primitive.rs:12`.

`[CURRENT]` Codec currently depends on core at
`crates/fornax-codec/Cargo.toml:9`, while TIFF currently has no workspace
dependency at `crates/fornax-tiff/Cargo.toml:6`.

`[TARGET]` Folding can reduce publish/version overhead but makes a future TIFF
raster depend on a larger codec package for basic buffer/error types.

`[TARGET]` This decision blocks the base release list and any dependency from
`fornax-tiff` to core.

## OPEN 3: move vendor codecs into `fornax-codec` now or later

`[TARGET]` Option A: move Sony ARW1/ARW2/ARW6/uncompressed/LJPEG functions
into `fornax-codec` now.

`[TARGET]` Option B: keep typed vendor functions in `fornax-raw-formats` until
a second vendor format shares them.

`[CURRENT]` Sony codec dispatch is in
`crates/fornax-arw/src/decode.rs:8` through
`crates/fornax-arw/src/decode.rs:18`, and the file is 460 LOC.

`[CURRENT]` ARW2 codec behavior depends on metadata curve knots at
`crates/fornax-arw/src/decode.rs:118`.

`[CURRENT]` `flashraw-codecs` is already a dependency of both ARW
(`crates/fornax-arw/Cargo.toml:7`) and DNG
(`crates/fornax-dng/Cargo.toml:11`).

`[CURRENT]` ARW is the only implemented vendor format at
`crates/fornax-arw/src/lib.rs:41`; the workspace membership can be checked with
`find crates -maxdepth 2 -name Cargo.toml -print | sort`, and future NEF/CR3
plans are at `docs/decoder/formats/nikon-nef.md:1` and
`docs/decoder/formats/canon-cr3.md:1`.

`[TARGET]` Immediate movement reduces duplicate future codec ownership but risks
putting camera metadata policy into a base package.

`[TARGET]` This decision blocks codec feature names, raw-formats layout, and
whether raw codecs can remain outside the base workspace.

## OPEN 4: `raster` as TIFF feature or separate crate

`[TARGET]` Option A: keep `raster` as an optional feature of `fornax-tiff`.

`[TARGET]` Option B: create a separate TIFF-raster crate.

`[CURRENT]` `fornax-tiff` has only `num_enum` and `strum` dependencies at
`crates/fornax-tiff/Cargo.toml:6`.

`[CURRENT]` DNG currently contains the strip/tile decode plan at
`crates/fornax-dng/src/dng_decoder/mod.rs:159` and compression mapping at
`crates/fornax-dng/src/dng_decoder/mod.rs:197`.

`[CURRENT]` Codec is a separate 2,634-LOC crate whose public request/output
API is raw-coupled today at `crates/fornax-codec/src/lib.rs:49`.

`[TARGET]` A feature keeps TIFF's public package count low but enlarges the
feature matrix and makes codec an optional dependency of TIFF.

`[TARGET]` A separate crate gives clearer ownership but adds another base
publication and potentially another wrapper type around `Directory`.

`[TARGET]` This decision blocks the final dependency graph, release order, and
whether geography consumers select TIFF raster through a feature.

## OPEN 5: naming and publish list

`[TARGET]` Option A: use the working name `fornax-raw-types`.

`[TARGET]` Option B: choose a broader name such as a raw-model or metadata
name if future types exceed the current list.

`[TARGET]` Publish-list option A: publish the full base and raw stack.

`[TARGET]` Publish-list option B: publish base plus the raw facade, keeping
internal raw crates unpublished.

`[CURRENT]` Workspace package and dependency inheritance are centralized at
`Cargo.toml:6` and `Cargo.toml:10`.

`[CURRENT]` `compare-demosaic` and `decoder-tracker` are already
`publish = false` at `crates/compare-demosaic/Cargo.toml:6` and
`crates/decoder-tracker/Cargo.toml:5`.

`[CURRENT]` `fornax-devtool` currently has no `publish = false` field;
`rg -n '^publish' crates/fornax-devtool/Cargo.toml` returns no match.

`[TARGET]` The target requires `fornax-devtool` and
`fornax-raw-devtool` to be `publish = false`.

`[TARGET]` This decision blocks manifests, semver commitments, release
automation, and compatibility-package strategy.

## Evidence gap 1: excluded Python binding

`[CURRENT]` `fornax-py` imports `fornax::libraw`,
`fornax::fornax_dalim`, and `fornax::Fornax` at
`crates/fornax-py/src/lib.rs:10`, `crates/fornax-py/src/lib.rs:11`, and
`crates/fornax-py/src/lib.rs:63`; current facade exports do not provide them
at `crates/fornax/src/lib.rs:1`.

`[CURRENT]` Its manifest requests workspace dependencies that are not listed
in the current root workspace table, including `numpy`, `pyo3`, and
`rmp-serde`, at `crates/fornax-py/Cargo.toml:14`,
`crates/fornax-py/Cargo.toml:15`, and
`crates/fornax-py/Cargo.toml:16`.

`[TARGET]` The intended Python API, migration target, and acceptance command
cannot be determined from the repository at this baseline.

`[TARGET]` This gap blocks a reliable estimate for keeping `fornax-py` in the
raw tools workspace.

## Evidence gap 2: ambiguous constant provenance

`[CURRENT]` The repository identifies JPEG XL 52546 as DNG 1.7 and 50000 as a
libtiff/GDAL extension at `docs/references/dng-compression.md:21` and
`docs/references/dng-compression.md:24`.

`[CURRENT]` The code and notes do not state a normative source for Deflate
value 8, lossy JPEG 34892, legacy JPEG XL 50002, or CFA photometric value
32803; `rg -n 'Deflate|34892|50002|CFA' crates/fornax-tiff docs/references`
shows implementation use but no normative citation for those four claims.

`[TARGET]` The final owning crate for those values cannot be decided without
checking authoritative TIFF 6.0, TIFF/EP, and DNG specifications.

`[TARGET]` This gap blocks Step 7 and the fully exact feature-to-compression
table.

## Evidence gap 3: physical two-workspace layout

`[TARGET]` The target mandates two workspaces but does not name their
directories or whether the repository root remains a meta-workspace.

`[CURRENT]` The current root uses `members = ["./crates/*"]` and inherits
package metadata at `Cargo.toml:3` and `Cargo.toml:6`.

`[CURRENT]` CI and validation assume one root and workspace-wide Cargo
commands at `.github/workflows/ci.yml:85` and
`scripts/agent-validation.sh:202`.

`[TARGET]` This gap blocks Step 9's exact scripts, CI topology, and developer
commands.

## Evidence gap 4: raw vendor features

`[TARGET]` The target says `fornax-raw` has no Cargo features but does not say
whether `fornax-raw-formats` should expose one feature per vendor.

`[CURRENT]` No current workspace crate exposes vendor-selection features;
`fornax-dng` and `fornax` have only metadata-related features at
`crates/fornax-dng/Cargo.toml:29` and `crates/fornax/Cargo.toml:14`, while
`rg -n '^\[features\]' crates/*/Cargo.toml` reports only those two manifests.

`[TARGET]` This gap blocks compile-time cost measurements and feature-matrix
tests for future NEF/CR3 work.

## Evidence gap 5: compatibility and deprecation policy

`[TARGET]` The target retires the old `fornax` facade and moves many public
paths but does not state whether a compatibility release or shim package is
required.

`[CURRENT]` The facade reexports core names, DNG, ARW, decoder, preprocessor,
and postprocessor at `crates/fornax/src/lib.rs:2` through
`crates/fornax/src/lib.rs:9`.

`[CURRENT]` CLI uses facade paths at `crates/fornax-cli/src/convert.rs:5` and
`crates/fornax-cli/src/config.rs:6`; tracker uses internal paths at
`crates/decoder-tracker/src/stages.rs:5`.

`[TARGET]` This gap blocks the external semver/deprecation plan and release
notes, but not the internal migration sequence.

## Evidence gap 6: external-converter adapter and benchmark ownership

`[TARGET]` The target names no replacement owner for the platform-gated Adobe
DNG Converter adapter or the Rawler benchmark wrapper.

`[CURRENT]` The DNC implementation is a private platform-gated decoder module
at `crates/fornax-decoder/src/lib.rs:3` and
`crates/fornax-decoder/src/dnc.rs:247`.

`[CURRENT]` The Rawler adapter exists only in
`crates/fornax-decoder/benches/rawler_wrapper.rs:11`, while the benchmark is
declared at `crates/fornax-decoder/Cargo.toml:6`.

`[TARGET]` The adapter could live with format/tooling code, and decoder
benchmarks could live with raw-formats or the facade.

`[TARGET]` This gap blocks the final layout of `fornax-decoder` source and
bench targets in Step 8.
