# RawTherapee demosaic reference integration

## Goal

Add RawTherapee 5.13 as a pinned comparison reference and exercise every Bayer
demosaic method that exists in both RawTherapee and Fornax. Prepare reference
adapters for IGV, EAHD, and HPHD and track their Fornax ports in `TODO.md`.
Dual demosaic variants are explicitly excluded.

## Scope

In scope:

- Conventional 2x2 RGB Bayer input only.
- RawTherapee source provisioning, adapters, backend dispatch, metadata, and
  tests.
- Comparisons for AHD, AMaZE, DCB, LMMSE, RCD, and VNG4.
- Direct adapter tests for IGV, EAHD, and HPHD.
- Reference notes, baseline, generated README, and `TODO.md` updates.

Out of scope:

- Fornax implementations of IGV, EAHD, or HPHD.
- Dual, AMaZE+Bilinear, AMaZE+VNG4, RCD+Bilinear, RCD+VNG4, DCB+Bilinear, or
  DCB+VNG4.
- X-Trans, four-color, or monochrome demosaicing.
- RawTherapee decoding, preprocessing, white balance, or output-color stages.

## Task checklist

- [ ] **Task 1:** Add backend metadata and reference-selection tests.
- [ ] **Task 2:** Add pinned source provisioning and RawTherapee adapters.
- [ ] **Task 3:** Register shared-algorithm comparisons.
- [ ] **Task 4:** Add reference documentation and TODO entries.
- [ ] **Task 5:** Refresh generated artifacts and run full validation.
- [ ] Pin RawTherapee release 5.13 at commit
  `1924179107995b01b73cdc7f4fe91e44fc8227a7` in the platform setup scripts and
  `build.rs`.
- [ ] Add a RawTherapee compatibility facade and adapters for AHD, AMaZE, DCB,
  LMMSE, RCD, VNG4, IGV, EAHD, and HPHD.
- [ ] Add the RawTherapee backend and stable algorithm IDs to the C++ shim and
  Rust FFI dispatch.
- [ ] Register RawTherapee cases for every shared Fornax/RawTherapee algorithm.
- [ ] Add direct reference tests and checksum snapshots for IGV, EAHD, and HPHD.
- [ ] Add RawTherapee reference and version metadata to canonical JSON and the
  generated README.
- [ ] Document RawTherapee source, parameters, compatibility differences, and
  GPL-3.0-or-later constraints.
- [ ] Add `[IGV]`, `[EAHD]`, and `[HPHD]` to `TODO.md`; update `[AMAZE]` to
  include both darktable and RawTherapee.
- [ ] Refresh snapshots, the canonical baseline, and generated README regions.
- [ ] Run all required validation and inspect the final diff.

## Task 1: Backend metadata and selection

**Files:** `crates/compare-demosaic/src/cases.rs`,
`crates/compare-demosaic/src/reference.rs`, `crates/compare-demosaic/src/run.rs`,
`crates/compare-demosaic/src/report.rs`, `crates/compare-demosaic/src/lib.rs`,
`crates/compare-demosaic/src/main.rs`, `crates/compare-demosaic/cpp/shim.{h,cpp}`,
`crates/compare-demosaic/build.rs`.

**Steps:**

1. Add a failing test asserting `ReferenceBackend::RawTherapee.library()`,
   `reference_names()`, and the source-link version map recognize
   `rawtherapee`.
2. Run `cargo test -p compare-demosaic source_links_match_packaged_versions`;
   expected: compile failure because `ReferenceBackend::RawTherapee` and the
   RawTherapee metadata do not exist.
3. Add RawTherapee version/commit constants, report metadata, backend dispatch,
   and FFI backend ID without registering cases yet.
4. Run the test; expected: pass.

## Task 2: Pinned sources and adapters

**Files:** `scripts/setup.{sh,ps1}`,
`crates/compare-demosaic/cpp/algos/rawtherapee*/**`,
`crates/compare-demosaic/cpp/algos/rawtherapee_*.{cpp,c}`,
`crates/compare-demosaic/cpp/shim.{h,cpp}`, `crates/compare-demosaic/build.rs`,
`crates/compare-demosaic/src/lib.rs`.

**Steps:**

1. Add failing direct-reference tests for IGV, EAHD, and HPHD using a valid
   RawTherapee algorithm ID on the 128x128 synthetic fixture. Assert non-empty
   RGB output, deterministic second output, and retained sampled Bayer codes.
2. Run `cargo test -p compare-demosaic rawtherapee`; expected: fail because the
   RawTherapee adapters and IDs are absent.
3. Materialize RawTherapee 5.13 source files through both setup scripts and add
   the compatibility facade plus adapter sources for AHD, AMaZE, DCB, LMMSE,
   RCD, VNG4, IGV, EAHD, and HPHD. Normalize split-green masks, use identity
   color matrices, scalar paths, and RawTherapee's 16-bit working scale.
4. Wire all adapter sources into `build.rs` and the shim.
5. Run `cargo test -p compare-demosaic rawtherapee`; expected: pass.

## Task 3: Shared algorithm comparisons

**Files:** `crates/compare-demosaic/src/cases.rs`,
`crates/compare-demosaic/src/lib.rs`,
`crates/compare-demosaic/src/snapshots/**`.

**Steps:**

1. Add a failing test asserting RawTherapee cases exist for AHD, AMaZE, DCB,
   LMMSE, RCD, and VNG4 and that no algorithm ID or case is named Dual.
2. Run the test; expected: fail because the six cases are not registered.
3. Register the six cases with commit-bearing source links and the existing
   Fornax runners. Use DCB `(0, true)`, LMMSE `(1)`, and comparison-safe tiling
   for AMaZE/RCD.
4. Run the test and the comparison crate suite; expected: pass aside from
   intentionally obsolete snapshots, which are reviewed and updated.

## Task 4: Documentation and backlog

**Files:** `TODO.md`, `docs/references/rawtherapee.md`,
`crates/compare-demosaic/AGENTS.md`, `crates/compare-demosaic/README.md`.

**Steps:**

1. Update hand-written README notes and reference-source setup guidance.
2. Document the pinned source, GPL-3.0-or-later constraint, compatibility
   facade, identity-matrix assumption, scalar path, method parameters, and
   exclusions.
3. Add `[IGV]`, `[EAHD]`, and `[HPHD]` to `TODO.md`; expand `[AMAZE]` to cover
   RawTherapee as well as darktable.
4. Run `git diff --check`; expected: exit 0.

## Task 5: Generated artifacts and validation

**Files:** `crates/compare-demosaic/baseline/canonical.json`,
`crates/compare-demosaic/README.md`, snapshots, and the plan checklist.

**Steps:**

1. Run a RawTherapee-only comparison; expected: six shared cases across four
   patterns and no Dual rows.
2. Refresh the full baseline and generated README; review the diff and do not
   waive failing rows.
3. Run the complete validation commands from the Validation section and record
   every outcome.
4. Inspect the final diff, mark completed checklist items, and commit the whole
   implementation as one semantic change.

## Files expected to change

- `scripts/setup.sh`, `scripts/setup.ps1`
- `crates/compare-demosaic/build.rs`
- `crates/compare-demosaic/cpp/algos/**`
- `crates/compare-demosaic/cpp/shim.{h,cpp}`
- `crates/compare-demosaic/src/**`
- `crates/compare-demosaic/README.md`
- `crates/compare-demosaic/baseline/canonical.json`
- `crates/compare-demosaic/AGENTS.md`
- `docs/references/rawtherapee.md`
- `docs/plans/chore-rawtherapee-demosaic-reference.md`
- `TODO.md`

## Validation

```bash
source scripts/setup.sh
cargo test -p compare-demosaic --all-features
cargo run --release -p compare-demosaic -- run --reference rawtherapee
cargo run --release -p compare-demosaic -- run
cargo run --release -p compare-demosaic -- readme
cargo run --release -p compare-demosaic -- readme --check
cargo run --release -p compare-demosaic -- diff \
  crates/compare-demosaic/baseline/canonical.json \
  target/compare-demosaic/canonical.json
cargo test --workspace --all-features
cargo fmt --check
cargo clippy --workspace --all-features --all-targets -- -D warnings -A clippy::too_many_arguments
git diff --check
```

## Decisions and notes

- RawTherapee 5.13 is the pinned release.
- The integration follows the existing LibRaw/darktable source-adapter pattern:
  materialize only required upstream files and compile them behind small
  compatibility headers.
- Scalar RawTherapee source branches are used to avoid a full GTK/GLib/FFTW
  dependency and keep the reference deterministic across supported platforms.
- The comparison supplies an identity camera matrix and no white balance.
- DCB uses `(0, true)` and LMMSE uses one iteration so the compared variants
  correspond to the existing LibRaw/darktable reference semantics.
- IGV, EAHD, and HPHD remain adapter-only until matching Fornax implementations
  exist.
