# DEC-CORPUS: Git LFS-backed Manual DNG Regression Tracker

## Goal

Replace the development-only Python downloader with a pinned raw.pixls.us Git
LFS submodule and a workspace `decoder-tracker` tracker. The tracker records a
reviewed, text-only baseline for every DNG in the pinned snapshot. Full-corpus
runs remain manual on the default branch.

## Scope

In scope:

- Add `external/raw.pixls.us` from
  `https://raw.pixls.us/data.lfs.git` and pin its accepted contents in the
  committed manifest.
- Add the `decoder-tracker` crate with `manifest`, `run`, `merge`, and `diff`
  subcommands.
- Add a canonical manifest at `crates/decoder-tracker/manifest.csv` and the DNG
  result baseline at `crates/decoder-tracker/baseline.csv`.
- Add a manual, matrix-sharded GitHub Actions workflow.
- Add `git-lfs` to the Pixi development environment.
- Validate corpus pointer metadata in `scripts/agent-validation.sh`.
- Remove the Python downloader, data Pixi environment, HTTP/aria2 dependencies,
  and stale acquisition documentation.
- Remove `[DEC-CORPUS]` from `TODO.md` after the baseline and manual verify are
  complete.

Out of scope:

- Committing or redistributing RAW payloads.
- Decoding non-DNG native formats before their decoder implementations land.
- Scheduled or pull-request-triggered full-corpus tests.
- Changing the existing `external/raw-images` fixture submodule.

## Task Checklist

- [x] Create the branch and worktree from `origin/main`.
- [x] Add failing tests for path normalization, metadata derivation, filtering,
  sharding bounds, stage transitions, metadata sanity, timeout/panic mapping,
  canonical CSV, merge completeness, diff classifications, and manifest checks.
- [x] Implement the Rust CLI and supporting modules until those tests pass.
- [x] Pin the LFS submodule contents in `manifest.csv` and generate the DNG
  manifest from
  `filelist.sha256`.
- [x] Add the manual sharded workflow and temporary branch bootstrap trigger.
- [x] Replace the data Pixi environment and downloader.
- [x] Update validation and architecture/corpus documentation.
- [x] Run local one-file failure and known-good fixture smoke tests.
- [x] Bootstrap the full DNG corpus, review the candidate baseline, and commit
  it.
- [x] Remove the temporary trigger and `[DEC-CORPUS]` after verification.
- [ ] Run `scripts/agent-validation.sh` to a clean result.
- [x] Commit, push, open one PR, and report its URL and validation evidence.

## Files Expected to Change

- `.gitmodules`, `external/raw.pixls.us`
- `Cargo.toml`, `Cargo.lock`, `pixi.toml`, `pixi.lock`
- `crates/decoder-tracker/manifest.csv`, `crates/decoder-tracker/baseline.csv`,
  `crates/decoder-tracker/src/**`
- `.github/workflows/decoder-tracker-corpus.yml`
- `scripts/agent-validation.sh`, `scripts/agent-validation-test.sh`
- `scripts/get_raw_samples.py` (deleted)
- `TODO.md`, `AGENTS.md`
- `docs/architecture.md`, `docs/decoder/architecture.md`,
  `docs/decoder/corpus.md`

## Validation

- `cargo test -p decoder-tracker`
- `cargo run -p decoder-tracker -- manifest --check`
- A local run over at most two DNG samples.
- Manual bootstrap at 20 shards, candidate review against the manifest, and a
  manual verify with empty diff.
- `scripts/agent-validation.sh`
- Confirm no RAW payload is tracked by Git and no payload artifact is uploaded.

## Decisions and notes

- Supported formats are filtered before deterministic SHA-256 ordering and
  sharding.
- The baseline row key is `(camera, format, path)`; `camera` already includes
  the vendor name when needed.
- Decoder stages are evaluated once each in order. Child-process timeout and
  panic handling turns decoder failures into baseline rows rather than
  infrastructure failures.
- The local full-run guard defaults to two files unless `CI=true` or
  `--allow-full` is present.
- Payload materialization happens before per-file timers start.
- The final workflow has only `workflow_dispatch`.
- CI dynamically allocates up to 20 balanced runners and retries failed jobs in
  place after transient GitHub/Pixi 504 failures. Workflow concurrency cancels
  superseded branch runs.
- The planned commit `174acd7...` was no longer present when implementation
  started. raw.pixls.us advertises one daily snapshot commit and replaces its
  history, so fresh checkouts follow the remote `master` branch with LFS
  smudging disabled. `manifest.csv` is the durable snapshot pin and
  `manifest --check` rejects uncommitted upstream changes.

## Adaptive revision tasks

### Task 1: Glob Filters And Dynamic Runner Sizing

- Add failing tests for `*`, `**`, `?`, repeated-glob OR behavior, combined
  filters, zero matches, and runner sizing at 5, 10, 11, 16, and 210 files.
- Add repeatable `--glob` to selection and CLI plumbing.
- Add a `select` command that writes a filtered manifest and prints its count.
- Compute `min(20, max(1, ceil(count / 10)))` runners and verify balanced
  modulo sharding.

### Task 2: DuckDB CSV Aggregation

- Add failing SQL integration tests for duplicate detection, selected-key
  completeness, selected-row diff, unchanged output, and partial upsert.
- Add `duckdb` to a tracker Pixi feature/environment and dev environment.
- Replace Rust merge/diff maps with checked-in SQL invoked through a small
  DuckDB command wrapper.
- Validate canonical CSV after DuckDB output.

### Task 3: Adaptive Manual Workflow

- Replace `mode` and `shard_count` inputs with one `filter` glob input whose
  case-safe default is `**/*.[dD][nN][gG]`.
- Plan from `selected-manifest.csv`, generate the dynamic matrix, and upload
  the selected manifest.
- Merge, diff, and upsert selected keys with DuckDB.
- Upload updated baseline/diff and fail when selected results differ; upload a
  bootstrap baseline when none exists; upload nothing when unchanged.

### Task 4: Corpus Refresh And Documentation

- Refresh the daily raw.pixls.us pointer snapshot and regenerate `manifest.csv`.
- Update corpus, architecture, validation, and operator documentation.
- Keep `prefix-dev/setup-pixi` as the Pixi installation path.

### Task 5: Full Bootstrap And Verification

- Run focused tests and the local filtered smoke test.
- Run the adaptive workflow to bootstrap the full accepted baseline.
- Review and commit the baseline, rerun for an unchanged result, then remove
  the temporary branch push trigger and `[DEC-CORPUS]`.
- Run `scripts/agent-validation.sh`; report GitHub/Pixi infrastructure failures
  separately from code failures.

### Adaptive revision decisions

- Glob matching is case-sensitive and segment-aware. The default uses character
  classes so it selects both `.DNG` and `.dng` paths.
- Runner count is `min(20, max(1, ceil(selected_count / 10)))`; modulo sharding
  balances rows across the resulting count.
- DuckDB CLI is supplied by the `duckdb-cli` Pixi package in the dev and tracker
  environments. Rust validates schemas and DuckDB output around the SQL calls.
- A selected-row difference writes a full updated baseline and diff artifact,
  then fails the workflow for review.
- `prefix-dev/setup-pixi` remains the Pixi bootstrap mechanism despite transient
  GitHub 504 responses.
- DuckDB SQL self-tests are optional through the manual `run_duckdb_tests`
  workflow input and default to disabled.
- Plan-time selection and aggregate merge/diff/upsert run through checked-in
  shell scripts backed directly by DuckDB CLI, avoiding Rust builds in those
  jobs.

### Verification evidence

- Bootstrap workflow `42477649926` completed successfully after retrying failed
  Pixi download jobs in place.
- Candidate baseline: 262 unique rows covering all 262 manifest paths; 220
  decoded and 209 passed handoff.
- Final manual-equivalent verification `42481636207` completed successfully,
  logged `outcome=unchanged`, and produced no `decoder-baseline-update`
  artifact.
- `cargo test -p decoder-tracker`: 37 tests passed.
- `pixi run -e tracker cargo test -q -p decoder-tracker duckdb_`: DuckDB SQL,
  selection, and aggregate shell tests passed.
