# Migration plan

## Ordering decision

`[TARGET]` The recommended order below differs from the initial suggested
order by moving duplicated TIFF helpers and neutral codec output ahead of the
core split. The evidence supports that change: these preparatory slices reduce
the reverse dependencies that make the core split large, while the codec
buffer change touches 27 files and 234 `RawMosaic` occurrences.

`[TARGET]` Every step is a separate shippable commit or pull request with
compatibility reexports where they are cheap.

`[TARGET]` Verification commands are named but were not run for this
documentation-only package.

## Step 1: consolidate TIFF tag/crop helpers

`[TARGET]` Crates touched: `fornax-tiff`, `fornax-dng`, `fornax-arw`.

`[TARGET]` Symbols moved: shared unsigned scalar/list access, active-area and
default-crop rectangle parsing, and crop-and-cast helpers currently duplicated
at `crates/fornax-dng/src/dng_decoder/mod.rs:340`,
`crates/fornax-arw/src/container.rs:236`,
`crates/fornax-dng/src/dng_decoder/process_metadata.rs:157`,
`crates/fornax-arw/src/metadata.rs:189`,
`crates/fornax-dng/src/dng_decoder/mod.rs:130`, and
`crates/fornax-arw/src/lib.rs:158`.

`[TARGET]` Callers changed: four source files in two crates; no workspace
consumer outside DNG/ARW needs a path change.

`[TARGET]` Existing coverage: 26 DNG tests across
`crates/fornax-dng/src/dng_decoder/mod.rs`,
`crates/fornax-dng/src/dng_decoder/process_metadata.rs`,
`crates/fornax-dng/src/metadata.rs`, `crates/fornax-dng/src/writer.rs`, and
`crates/fornax-dng/tests/directory_metadata.rs`; 10 ARW tests across
`crates/fornax-arw/src/tests.rs`, `crates/fornax-arw/src/decode.rs`, and
`crates/fornax-arw/src/sony.rs`; 10 TIFF tests under
`crates/fornax-tiff/src` and `crates/fornax-tiff/tests`.

`[TARGET]` Benchmarks: no dedicated helper benchmark; the DNG writer and
decoder benches exercise dependent paths.

`[TARGET]` Verify with `cargo test -p fornax-tiff -p fornax-dng -p fornax-arw`,
then `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps -p fornax-tiff -p fornax-dng -p fornax-arw`.

`[TARGET]` Risk: medium, because conversion width/type behavior can change
subtly. Effort estimate: small.

`[TARGET]` Roll back by reverting the helper extraction commit; the old private
helpers remain in the prior revision.

## Step 2: give codec a neutral output buffer

`[TARGET]` Crates touched: `fornax-core`, `fornax-codec`, `fornax-dng`,
`fornax-arw`, `fornax-devtool`, `fornax-decoder`, `decoder-tracker`,
`fornax-cli`, and `fornax`.

`[TARGET]` Symbols moved: codec request/output types stop naming
`fornax_core::RawMosaic` at `crates/fornax-codec/src/lib.rs:49`; the neutral
buffer placeholder from OPEN 1 becomes codec output. A temporary
`RawMosaic::from_neutral`/`into_neutral` adapter may live in `fornax-core`
until `fornax-raw-types` exists.

`[TARGET]` Callers changed: 27 workspace files contain `RawMosaic`, with 234
token occurrences; direct source users include
`crates/fornax-codec/src/samples.rs:1`,
`crates/fornax-dng/src/dng_decoder/mod.rs:9`, and
`crates/fornax-arw/src/lib.rs:17`.

`[TARGET]` Existing coverage: 25 codec tests in
`crates/fornax-codec/src/ljpeg.rs`,
`crates/fornax-codec/src/uncompressed.rs`,
`crates/fornax-codec/tests/decode_contract.rs`, and
`crates/fornax-codec/tests/encode_contract.rs`; 26 DNG tests; 10 ARW tests; 3
decoder tests; 6 CLI tests.

`[TARGET]` Benchmarks: `crates/fornax-decoder/benches/decoder.rs` and
`crates/fornax-dng/benches/writer.rs`.

`[TARGET]` Verify with
`cargo test -p fornax-codec -p fornax-dng -p fornax-decoder -p fornax-cli`,
and compile the decoder bench without running it with
`cargo bench -p fornax-decoder --no-run`.

`[TARGET]` Risk: high, because `RawMosaic` is the widest shared raw type and
the neutral buffer shape is OPEN 1. Effort estimate: medium to large.

`[TARGET]` Roll back by reverting the buffer commit; keep a compatibility
conversion layer in the same commit if callers are staged across commits.

## Step 3: split neutral core from raw types

`[TARGET]` Crates touched: new `fornax-raw-types`, `fornax-core`, and every
crate that imports current core domain/types/metadata/protocols.

`[TARGET]` Symbols moved: `BayerChannel`, `BayerPattern`, `RawFormat`,
`RawMosaic`, `CroppedMosaic`, `NormalizedMosaic`, raw errors, `ProcessMetadata`,
`Levels`, color calibration types, and the decoder/pre/post protocols listed
in [03-mapping.md](03-mapping.md).

`[TARGET]` Callers changed: 10 current crates directly depend on
`fornax-core`; 54 workspace source/test/bench files contain core-path imports
or facade aliases.

`[TARGET]` Existing coverage: 10 core tests split across
`crates/fornax-core/src/bayer.rs` and
`crates/fornax-core/src/metadata.rs`; all 282 workspace unit/integration tests
provide downstream coverage.

`[TARGET]` Benchmarks: decoder bench and all stage benchmarks compile against
the moved types.

`[TARGET]` Verify with
`cargo test --workspace --all-features --exclude compare-demosaic`, then the
existing validation script after all crates import the new crate directly.

`[TARGET]` Risk: high, because this is the largest public-path move. Effort
estimate: large.

`[TARGET]` Roll back by reverting the split commit; `fornax-core` should carry
temporary reexports during the transition to make a partial rollout reversible.

## Step 4: merge preprocessing and postprocessing

`[TARGET]` Crates touched: `fornax-preprocessor`, `fornax-postprocessor`, new
`fornax-raw-pipeline`, `fornax`, `fornax-cli`, and `compare-demosaic`.

`[TARGET]` Symbols moved: `Preprocessor`,
`crates/fornax-preprocessor/src/lib.rs:7`; `Demosaic`, `LmmseMode`,
`OutputColorSpace`, `PostProcessor`, and the implementations under
`crates/fornax-postprocessor/src`. `IDemosaic`/`LazyDemosaic` move to
`fornax-raw-types` in the same or immediately following slice.

`[TARGET]` Callers changed: four source paths in three crates: facade
reexports at `crates/fornax/src/lib.rs:1` and
`crates/fornax/src/lib.rs:8`, CLI at
`crates/fornax-cli/src/config.rs:7`, and compare at
`crates/compare-demosaic/src/reference.rs:6`.

`[TARGET]` Existing coverage: 4 preprocessor tests in
`crates/fornax-preprocessor/src/lib.rs`; 85 postprocessor tests under
`crates/fornax-postprocessor/src` and
`crates/fornax-postprocessor/tests/protocol.rs`; 56 compare tests; 6 CLI tests.

`[TARGET]` Benchmarks: `crates/fornax-preprocessor/benches/preprocessor.rs`
and `crates/fornax-postprocessor/benches/demosaic.rs`.

`[TARGET]` Verify with
`cargo test -p fornax-preprocessor -p fornax-postprocessor -p fornax-cli`,
then run the comparison package checks only when its C/C++ environment is
available.

`[TARGET]` Risk: medium-high, because 8,496 LOC and 89 tests move, although
algorithm behavior should remain unchanged. Effort estimate: large.

`[TARGET]` Roll back by restoring separate package roots; retain reexport
shims in the new crate so consumers can switch paths independently.

## Step 5: move strip/tile execution to TIFF raster

`[TARGET]` Crates touched: `fornax-tiff`, `fornax-codec`, `fornax-dng`, and
`fornax-arw`.

`[TARGET]` Symbols moved: DNG strip/tile layout and decode-plan construction
from `crates/fornax-dng/src/dng_decoder/mod.rs:191`; `Compression` tag mapping
from `crates/fornax-dng/src/dng_decoder/mod.rs:197`; generic offset/count
layout helpers from `crates/fornax-arw/src/container.rs:183`.

`[TARGET]` Callers changed: DNG and ARW only; five current files directly use
compression/layout constants.

`[TARGET]` Existing coverage: 10 TIFF tests, 26 DNG tests, 10 ARW tests, 36
tracker tests, and 6 CLI tests.

`[TARGET]` Benchmarks: `crates/fornax-decoder/benches/decoder.rs` and
`crates/fornax-dng/benches/writer.rs`.

`[TARGET]` Verify with
`cargo test -p fornax-tiff -p fornax-codec -p fornax-dng -p fornax-arw -p decoder-tracker`.

`[TARGET]` Risk: medium, because DNG-specific compression identity must not
force `fornax-tiff` to depend on `fornax-dng`. Effort estimate: medium.

`[TARGET]` Roll back by restoring DNG-local layout mapping.

## Step 6: move vendor codecs if OPEN 3 chooses it

`[TARGET]` Crates touched: `fornax-codec`, `fornax-raw-formats`, and their
callers.

`[TARGET]` Symbols moved: typed vendor functions and feature gates for Sony
ARW1, ARW2, ARW6, uncompressed/12-bit, and Sony LJPEG currently implemented
from `crates/fornax-arw/src/decode.rs:8` through
`crates/fornax-arw/src/decode.rs:18`.

`[TARGET]` Callers changed: one ARW decode-dispatch source file; no external
caller needs a second change if `fornax-raw-formats` reexports typed codec
functions.

`[TARGET]` Existing coverage: 10 ARW tests and 25 codec tests.

`[TARGET]` Benchmarks: decoder bench is the relevant end-to-end measurement.

`[TARGET]` Verify with `cargo test -p fornax-codec -p fornax-arw` before the
ARW crate is renamed.

`[TARGET]` Risk: medium if this waits for a second vendor, high if moved now
without a second user because the generic codec crate gains vendor metadata
policy. Effort estimate: medium.

`[TARGET]` Roll back by keeping vendor dispatch in `fornax-raw-formats` and
exporting typed codec helpers only.

## Step 7: move DNG constants and open the enums

`[TARGET]` Crates touched: `fornax-tiff`, `fornax-dng`, `fornax-arw`,
`decoder-tracker`, and `fornax-cli`.

`[TARGET]` Symbols moved: DNG-owned compression and photometric values from
`crates/fornax-tiff/src/constants.rs:85` and
`crates/fornax-tiff/src/constants.rs:108`; unknown tag IDs and value
representations remain preserved through open enums.

`[TARGET]` Callers changed: `TiffTag` appears in 16 workspace files,
`PhotometricInterpretation` in 8, and `CompressionMethod` in 3.

`[TARGET]` Existing coverage: 10 TIFF tests, 26 DNG tests, 10 ARW tests, 36
tracker tests, and 6 CLI tests.

`[TARGET]` Benchmarks: decoder and DNG writer benches.

`[TARGET]` Verify with
`cargo test -p fornax-tiff -p fornax-dng -p fornax-arw -p decoder-tracker -p fornax-cli`.

`[TARGET]` Risk: high, because an incomplete open-value design can silently
drop unknown fields. Effort estimate: medium.

`[TARGET]` Roll back by restoring the typed variants and compatibility aliases.

## Step 8: merge format decoders and retire the old facade

`[TARGET]` Crates touched: `fornax-arw`, `fornax-decoder`, new
`fornax-raw-formats`, new `fornax-raw`, `fornax-cli`, `decoder-tracker`, and
`compare-demosaic`.

`[TARGET]` Symbols moved: `ArwImage`, DNC adapter, and closed `Decoder`
dispatch. `Decoder` becomes the raw facade entry point; the old `fornax`
package is retired or retained only as a compatibility shell.

`[TARGET]` Callers changed: four direct orchestration consumers
(`fornax-cli`, `decoder-tracker`, `compare-demosaic`, and excluded
`fornax-py`) plus public users of the old facade.

`[TARGET]` Existing coverage: 10 ARW tests, 3 decoder tests, 26 DNG tests, 6
CLI tests, 36 tracker tests, 56 compare tests, and 89 pipeline tests.

`[TARGET]` Benchmarks: decoder bench plus pre/demosaic benches through facade
call sites.

`[TARGET]` Verify with
`cargo test --workspace --all-features --exclude compare-demosaic`, then the
existing compare check when its native environment is available.

`[TARGET]` Risk: high, because the closed enum and facade package path are
public API and are consumed by tools. Effort estimate: large.

`[TARGET]` Roll back by reintroducing the old facade package as a reexport
shim over `fornax-raw`.

## Step 9: split into two workspaces

`[TARGET]` Crates touched: every crate manifest and the root workspace
manifest.

`[TARGET]` Symbols moved: no Rust symbols; manifests move to base/raw
workspaces, workspace dependencies are duplicated or centralized per
workspace, and raw workspace base dependencies switch to versions plus local
`[patch.crates-io]`.

`[TARGET]` Callers changed: zero Rust source callers; five known
command/config files pin the old workspace shape:
`.github/workflows/ci.yml`, `.github/workflows/decoder-tracker-corpus.yml`,
`scripts/agent-validation.sh`, `scripts/agent-validation-test.sh`, and
`pixi.toml`.

`[TARGET]` Existing coverage: all 282 workspace tests.

`[TARGET]` Benchmarks: all four benchmark targets must still compile.

`[TARGET]` Verify with base and raw workspace command fixtures plus
`scripts/agent-validation.sh`; the script itself must first be updated for the
new workspace roots.

`[TARGET]` Risk: high, because Cargo workspace inheritance, version ordering,
and `[patch]` wiring can fail only in release/CI contexts. Effort estimate:
large.

`[TARGET]` Roll back by restoring the single workspace manifest while keeping
crate-level changes.

## Step 10: housekeeping and compatibility cleanup

`[TARGET]` Crates touched: raw tools and `default-members`; documentation-only
for live README/AGENTS updates.

`[TARGET]` Symbols moved: no runtime symbols; remove compatibility aliases
after a deprecation window, exclude `compare-demosaic` from default members,
resolve `fornax-py`, and refresh docs.

`[TARGET]` Callers changed: zero runtime symbols; CI path filters, package
commands, Python build scripts, and the 89-file documentation inventory are
reviewed here.

`[TARGET]` Existing coverage: all 282 tests under the new layout.

`[TARGET]` Benchmarks: compile all bench targets with `--no-run`.

`[TARGET]` Verify with the final workspace split equivalent of
`scripts/agent-validation.sh`, plus `git diff --check`.

`[TARGET]` Risk: medium, because stale commands and docs can survive a green
Rust build. Effort estimate: medium.

`[TARGET]` Roll back each housekeeping commit independently; compatibility
aliases should remain until this step.

## Risk register

| Risk | Evidence | Severity | Mitigation |
| --- | --- | --- | --- |
| TIFF raster can create a cycle | DNG depends on TIFF at `crates/fornax-dng/Cargo.toml:14`; target raster maps compression IDs used by DNG | High | Keep DNG constants out of TIFF and pass open numeric IDs |
| Neutral buffer drives broad churn | `RawMosaic` occurs in 27 files and 234 tokens; codec imports it at `crates/fornax-codec/src/lib.rs:49` | High | Land adapter conversions first and preserve shape/band invariants with tests |
| Public path breakage | Core exports at `crates/fornax-core/src/lib.rs:9`; facade reexports at `crates/fornax/src/lib.rs:2` | High | Stage deprecations/reexports and announce package rename |
| Vendor codec placement can couple codec to vendor metadata | ARW codec selects ARW2 curve from metadata at `crates/fornax-arw/src/decode.rs:118` | Medium-high | Keep typed payload function separate from metadata derivation; defer on OPEN 3 |
| Non-TIFF vendor formats may not fit DNG metadata language | `ArwImage` already synthesizes DNG-style `Directory` at `crates/fornax-arw/src/lib.rs:112`; CR3 is planned at `docs/decoder/formats/canon-cr3.md:1` but has no implementation crate | Medium | Define a vendor-neutral metadata adapter boundary before CR3 |
| CLI and tracker import current paths | `crates/fornax-cli/src/convert.rs:5`, `crates/decoder-tracker/src/stages.rs:5` | Medium | Move orchestration consumers in the same PR as facade/type moves |
| Compare tool compiles pinned native references | `crates/compare-demosaic/build.rs:91` | Medium | Keep it excluded from default members and avoid it in fast base CI |
| CI hard-codes paths and packages | `.github/workflows/ci.yml:38`, `.github/workflows/ci.yml:149`, `.github/workflows/ci.yml:186` | Medium | Update change filters and matrix in the workspace-split PR |
| Validation hard-codes paths and package names | `scripts/agent-validation.sh:200`, `scripts/agent-validation.sh:208`, `scripts/agent-validation.sh:224` | Medium | Make workspace roots/package names variables and add command-level tests |
| Generated docs/table data pin crate names | `crates/compare-demosaic/src/readme.rs:8`, `crates/decoder-tracker/src/manifest.rs:9` | Medium | Regenerate with existing tooling and review diffs |
| Two-workspace release order | Raw workspace depends on base by version; base development uses local patch wiring | High | Publish base non-optional crates first and test from a clean registry/patch fixture |
| `fornax-py` is already incompatible | `crates/fornax-py/src/lib.rs:10` references `fornax::fornax_dalim`; facade has no such export at `crates/fornax/src/lib.rs:1` | Medium | Treat Python revival as separate work with its own acceptance tests |
| Feature-disabled errors can diverge | Target requires `UnsupportedCompression`; current dispatch has no features at `crates/fornax-codec/src/lib.rs:90` | Medium | Add compile-matrix tests for every disabled feature |
| Open value representation can lose data | Current `Value` enumerates known TIFF field types at `crates/fornax-tiff/src/value.rs:23` | High | Specify unknown-type bytes before changing parser/writer |
| Package/version inheritance breaks across workspaces | Current crates use workspace package/dependency inheritance at `Cargo.toml:6` and `Cargo.toml:10` | High | Introduce separate workspace roots with explicit shared manifests and release-order checks |

`[TARGET]` The three highest-risk areas are the neutral-buffer/API cutover,
the TIFF raster/DNG constant boundary, and the two-workspace release wiring.
