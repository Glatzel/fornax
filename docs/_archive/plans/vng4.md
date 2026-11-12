# VNG4 implementation plan

## Goal

Add a native VNG4 demosaic target that keeps the two green CFA phases separate
through interpolation and averages them only when packing RGB, then validate it
against LibRaw 0.22.3's `four_color_rgb = 1` VNG path using VNG's unchanged
native GBRG limits.

## Scope

**In scope:** the public `Demosaic::Vng4` selector, VNG4 runner and benchmark
registration, a dedicated LibRaw VNG4 comparison adapter and case, focused
native and comparison tests, canonical comparison results, generated README
content and notes, VNG reference documentation, and the `[VNG4]` backlog
completion record.

**Out of scope:** new Bayer layouts, X-Trans, four-color camera sensors,
changes to the robust comparison limits, waivers, and changes to the existing
VNG behavior or LibRaw non-four-color comparison.

## Task checklist

- [x] Add a failing postprocessor test proving `Demosaic::Vng4` is a distinct
      selector that preserves measured Bayer samples.
- [x] Implement `Demosaic::Vng4` by reusing the existing split-green VNG kernel.
- [x] Add a failing comparison test proving LibRaw's VNG4 adapter uses
      `four_color_rgb = 1` and that the native GBRG row passes every unchanged
      robust limit.
- [x] Extend the LibRaw shim with VNG4 algorithm dispatch and a table-driven
      comparison case.
- [x] Register VNG4 in the postprocessor benchmark registry.
- [x] Run focused tests, then the VNG4 comparison with artifacts and inspect
      verdict, channels, sampled sites, and the worst pixels.
- [x] Refresh only intended comparison snapshots, canonical baseline rows, and
      generated README content; write the algorithm note.
- [x] Document the VNG versus VNG4 green-plane distinction and pinned sources.
- [x] Remove `[VNG4]` from `TODO.md` after all acceptance checks pass.
- [x] Run the repository validation suite and inspect the final diff.

## Files expected to change

- `TODO.md`
- `crates/fornax-raw-pipeline/src/demosaic.rs`
- `crates/fornax-raw-pipeline/src/demosaic/vng.rs`
- `crates/fornax-raw-pipeline/README.md`
- `crates/fornax-raw-pipeline/benches/algorithms.json`
- `crates/fornax-raw-pipeline/benches/support/demosaic_registry.rs`
- `crates/compare-demosaic/cpp/shim.cpp`
- `crates/compare-demosaic/cpp/shim.h`
- `crates/compare-demosaic/cpp/algos/libraw_vng.cpp`
- `crates/compare-demosaic/src/cases.rs`
- `crates/compare-demosaic/src/lib.rs`
- `crates/compare-demosaic/src/snapshots/*VNG4*`
- `crates/compare-demosaic/baseline/canonical.json`
- `crates/compare-demosaic/README.md`
- `docs/references/vng.md`
- `docs/plans/vng4-native.md`

## Interfaces

- Add `Demosaic::Vng4` and `AnyLazy::Vng4(DenseLazy<T>)`.
- Keep `DemosaicVng::lazy` unchanged; VNG4 selects the same split-green kernel
  and therefore has the same 8x8 minimum input size.
- Add `run_vng4` to the comparison runner registry as `Demosaic::Vng4`.
- Add LibRaw comparison algorithm ID `7`, implemented by
  `ReferenceDemosaicer::run_vng4`.
- Register one `VNG4` case against LibRaw with `Tolerance::ROBUST`.

## Validation

```bash
cargo test -p fornax-raw-pipeline vng
cargo test -p compare-demosaic native_vng4_meets_reference_limits
cargo run --release -p compare-demosaic -- run \
  --algorithm vng4 --pattern GBRG --artifacts --out target/vng4
cargo insta test -p compare-demosaic
cargo run --release -p compare-demosaic -- run
cp target/compare-demosaic/canonical.json \
  crates/compare-demosaic/baseline/canonical.json
cargo run --release -p compare-demosaic -- readme
cargo run --release -p compare-demosaic -- readme --check
cargo run --release -p compare-demosaic -- diff \
  crates/compare-demosaic/baseline/canonical.json \
  target/compare-demosaic/canonical.json
cargo test --workspace --all-features
cargo +nightly fmt --check
cargo clippy --workspace --all-features --all-targets -- -D warnings -A clippy::too_many_arguments
markdownlint-cli2 crates/compare-demosaic/README.md \
  crates/fornax-raw-pipeline/README.md docs/references/vng.md \
  docs/plans/vng4-native.md
git -c core.whitespace=cr-at-eol diff --check
```

The native GBRG VNG4 row must have MAE <= 2, P99 <= 64, and a high-error
fraction above 1024 codes <= 0.001. No numeric limit or waiver may change.

## Decisions and notes

- LibRaw implements this mode by setting `four_color_rgb = 1` before
  `pre_interpolate()`, which raises `colors` to four for
  `vng_interpolate()`, then averages channels 1 and 3 when `mix_green` is set
  after interpolation.
- Fornax already represents that interpolation state as split green channels 1
  and 3 and packs their midpoint into RGB green. VNG4 therefore reuses the
  existing VNG kernel instead of duplicating the phase tables.
- The existing VNG comparison against LibRaw remains informational because its
  adapter sets `four_color_rgb = 0`; VNG4 is the acceptance comparison for the
  four-color LibRaw mode.
- darktable's VNG bridge already behaves like split-green/four-color VNG, so
  the new public target is still restricted to conventional 2x2 RGB Bayer.
- Reference behavior is verified from LibRaw commit
  `cf02d66eb5c19d4e46df7aee0582e47a4918509a`, especially
  `src/demosaic/misc_demosaic.cpp` and
  `src/postprocessing/dcraw_process.cpp`. LibRaw is LGPL-2.1/CDDL and is used
  only as a behavioral oracle; no source is copied into the Rust implementation.
- The full comparison run added four VNG4 rows and did not change any existing
  row. Every VNG4 row has zero error, including the native GBRG acceptance row.
- `cargo test --workspace --all-features` passes. The documented workspace
  Clippy command allows the pre-existing `clippy::too_many_arguments` lint and
  passes. Nightly rustfmt passes; stable rustfmt cannot honor the repository's
  nightly-only configuration and reports unrelated formatting.
  `markdownlint-cli2` is not installed in this environment.
