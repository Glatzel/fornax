# Workspace refactor baseline

This archived workspace-refactor record documents Workspace refactor baseline.

## Goal

Record reproducible pre-refactor measurements and golden decode outputs so each
later refactor PR can prove that decoder output, cropping, process metadata, and
DNG metadata remain unchanged.

This PR covers only step 1 of the agreed sequence. It must not merge the
pre/post processors, change codec output, move TIFF raster logic, move DNG
constants, merge format crates, split workspaces, or publish crates.

## Scope

In scope:

- verify the advisory analysis against the current code and primary specifications;
- add a deterministic decoder-tracker command for golden hashes;
- commit a small pointer-only selected corpus manifest and golden hash CSV;
- record baseline build, test, size, dependency, and hash measurements;
- record the corrections and unresolved decisions that affect later steps.

Out of scope:

- production decoder behavior changes;
- changes to `baseline.csv`, `manifest.csv`, or the decoder-tracker README table;
- resolving the neutral buffer shape, vendor codec placement, or crate names;
- steps 2 through 6.

## Task checklist

- [x] Verify advisory claims with code evidence and primary web specifications.
- [x] Write failing tests for canonical raw, cropped, process, and DNG metadata hashes.
- [x] Implement the hash model and CSV check/write behavior.
- [x] Add a `decoder-tracker hash` command using a selected manifest and transient LFS payloads.
- [x] Select representative DNG and ARW cases from the committed corpus snapshot.
- [x] Generate and review the golden hash CSV.
- [x] Record baseline measurements and advisory corrections.
- [x] Run task tests, repository validation, and inspect the complete diff.
- [x] Open one PR and stop before step 2.

## Files expected to change

- `Cargo.lock`
- `crates/decoder-tracker/Cargo.toml`
- `crates/decoder-tracker/src/lib.rs`
- `crates/decoder-tracker/src/main.rs`
- `crates/decoder-tracker/src/golden.rs`
- `crates/decoder-tracker/tests/cli.rs`
- `crates/decoder-tracker/tests/golden.rs`
- `crates/decoder-tracker/golden/selected-manifest.csv`
- `crates/decoder-tracker/golden/decode-hashes.csv`
- `crates/decoder-tracker/AGENTS.md`
- `crates/decoder-tracker/README.md`
- `docs/decoder/corpus.md`
- `docs/_archive/workspace-refactor/baseline-findings.md`
- `docs/_archive/workspace-refactor/baseline-plan.md`

## Validation

- `cargo test -p decoder-tracker`
- `pixi run -e tracker cargo test -q -p decoder-tracker duckdb_`
- `cargo run -p decoder-tracker -- hash --manifest crates/decoder-tracker/golden/selected-manifest.csv --check crates/decoder-tracker/golden/decode-hashes.csv`
- `scripts/agent-validation.sh`
- `git diff --check`

The selected corpus payloads are transient. Only pointer metadata and hashes
may be committed.

## Decisions and notes

- The maintainer confirmed that `fornax` is unpublished, so baseline work does
  not need compatibility shims.
- The DNC fallback placement and the reduced refactor scope are maintainer
  decisions, not conclusions inferred from code.
- Neutral buffer shape, vendor codec placement, and crate names remain open and
  must not be implemented in this PR.
- Golden rows will hash raw mosaic samples, cropped `f32` samples, normalized
  process metadata, and DNG metadata. This is stricter than sample hashes alone
  and covers generic tag-accessor and raster extraction steps.
- Golden paths and hashes come from the committed pointer manifest. The command
  materializes selected LFS objects but never writes them into Git.

## Baseline observations

- Baseline commit: `9b9c1b284ff5f4fe9ef1c8ff41eb6c3cff17b8d2`.
- Workspace tests: 251 passed across 35 suites.
- Test execution time reported by Cargo: 5.97 seconds.
- Clean test/build wall time in a fresh target directory: 58.78 seconds.
- Clean release `fornax-cli` build: 68.84 seconds, 171 crates compiled.
- Release `fornax` binary size: 15,297,048 bytes.
- Release `decoder-tracker` binary size: 11,365,392 bytes.
- No-op release rebuild after the clean CLI build: 0.26 seconds.
- `cargo tree -p fornax-cli -e normal` currently reports 385 lines.
- `cargo tree -p fornax-tiff -e normal` currently reports 27 lines.
- `cargo tree -p fornax-codec -e normal` currently reports 151 lines.

## Advisory verification summary

- Confirmed with current code: raw pipeline preparation is mechanical; codec
  still returns `RawMosaic`; generic scalar/list access is duplicated; DNG and
  ARW active-area/crop logic have different tag semantics; pre/post processors
  have identical direct workspace dependencies.
- Corrected: `crop_and_cast` does not interpret DNG tags; it only crops and
  casts an image. It is not evidence for moving DNG policy into TIFF.
- Corrected: API breakage is less costly because the maintainer confirmed no
  crates.io publication. No compatibility shim is required for this refactor.
- Verified from primary sources: compression 34892 and 52546 are defined by
  DNG; 50000 and 50002 are self-assigned TIFF extensions rather than
  registered DNG values. This remains recorded for step 6 and is not acted on
  in this PR.
