# 0.4.0 Release Readiness Plan

## Goal

Prepare the decoder release evidence against FlashRaw 0.4.0, make the test
golden set cover every supported native format plus an ARW6 regression, and
move full golden-payload validation from routine local checks to CI.

## Scope

In scope:

- Archive merged, completed plans under `docs/archive/plans/`.
- Update the benchmark-only FlashRaw reference from 0.3.1 to 0.4.0.
- Re-run the raw-decode comparison and review both correctness and timing.
- Make the test golden manifest cover DNG, ARW, CR3, NEF, RAF, 3FR, RW2, and
  IIQ, plus an explicitly lossy Sony ARW6 regression row.
- Keep lossless-only coverage and performance-manifest rules separate from the
  broader correctness set.
- Replace the local full golden-payload hash pass with a cheap metadata/schema
  check; CI remains responsible for materializing and hashing every test row.

Out of scope:

- Production decoder optimization without measured evidence.
- A raw.pixls.us snapshot refresh or changes to RAW payload ownership.
- Expanding the conventional 2x2 RGB Bayer pipeline contract.
- Publishing a Fornax crate or creating the FlashRaw upstream release.

## Task Checklist

- [x] Create an isolated worktree from `origin/main` and confirm the tracker
  baseline passes (76 tests).
- [x] Archive plans whose work is merged and completed, add historical status
  headers, and update the archive/documentation indexes.
- [x] Add failing golden-set tests for supported-format coverage and explicit
  passing lossy regression rows.
- [x] Implement the golden-set policy and add the missing RAF, 3FR, RW2, IIQ,
  and ARW6 rows with release-profile hashes.
- [x] Move full materialized-hash validation to CI and keep `golden-check` as
  the local structural check.
- [x] Update FlashRaw to 0.4.0, adapt any API changes, and verify raw output
  equality before timing.
- [x] Measure FlashRaw 0.4.0 against Fornax at 1T and 16T, inspect every
  reported row, and record a dated snapshot plus current improvement priorities.
- [x] Run task-specific tests, repository validation, and inspect the complete
  diff.
- [x] Commit, push, and open one semantic PR:
  [#773](https://github.com/Glatzel/fornax/pull/773).
- [x] Remove superseded active FlashRaw comparison CSVs and point the decoder
  reference index at the 0.4.0 snapshot.

## Files Expected To Change

- `Cargo.lock`
- `crates/fornax-raw/Cargo.toml`
- `crates/fornax-raw/benches/`
- `crates/decoder-tracker/src/golden_set.rs`
- `crates/decoder-tracker/tests/`
- `crates/decoder-tracker/golden/`
- `.github/workflows/ci.yml`
- `scripts/agent-validation.sh`
- `docs/archive/README.md`
- `docs/archive/plans/`
- `docs/plans/`
- `docs/reference/decoders/reference/flashraw.md`
- `docs/reference/decoders/perf/flashraw-vs-fornax.md`
- `docs/reference/decoders/perf/data/`

## Validation

```bash
cargo test -p decoder-tracker
cargo test -p decoder-tracker --lib duckdb_
cargo run -p decoder-tracker -- golden-check
cargo run --release -p decoder-tracker -- hash \
  --manifest crates/decoder-tracker/golden/test-manifest.csv \
  --check crates/decoder-tracker/golden/decode-hashes.csv
cargo bench -p fornax-raw --no-run
FORNAX_PROBE_DECODERS=fornax,flashraw RAYON_NUM_THREADS=1 \
  cargo run --release -p fornax-raw --bench flashraw_probe
scripts/agent-validation.sh
git diff --check
```

The explicit release-profile hash command is release evidence to run once after
the new rows are selected. Routine `scripts/agent-validation.sh` runs only the
cheap `golden-check`; CI materializes the corpus and validates the full test
manifest with `FORNAX_GOLDEN_CORPUS_REQUIRED=1`.

## Decisions And Notes

- “Supported format” means a format with at least one fully passing
  conventional 2x2 RGB Bayer row in the accepted baseline. Unsupported NRW
  rows in the corpus do not count.
- A fully passing lossy/ambiguous regression row is valid in the test set when
  explicitly admitted, but remains excluded from lossless coverage selection and
  the performance set.
- Golden payload validation is expensive and host-independent in CI, so local
  validation checks manifest metadata, pointers, schemas, hash-row presence, and
  uncovered decoder paths without fetching RAW payloads.
- Timing evidence remains host-specific. The committed report records method,
  corpus/version provenance, observed values, and ranked follow-up work rather
  than a portable timing baseline.
- FlashRaw 0.4.0 is timing-neutral relative to 0.3.1 for uncancelled decode.
  The median paired ratio was `1.005x` at one thread and `0.995x` at sixteen;
  no comparable sample moved by 5%.
- `perf` on Nokia Lumia 1020 and Leica SL2 attributes most samples to the
  current DNG OpcodeList1 full-plane `f32` conversion/application/rounding
  path. That is now the first-ranked decoder improvement candidate.
- `scripts/agent-validation.sh` passes after correcting the stale
  `pixi run -e tracker` references to the existing `dev` environment.
- Active performance data now contains only the current 0.4.0 per-sample and
  version-comparison snapshots plus the stable input matrix.
