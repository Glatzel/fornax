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
- [ ] Bootstrap the full DNG corpus, review the candidate baseline, commit it,
  and verify an empty diff.
- [ ] Remove the temporary trigger and `[DEC-CORPUS]` after verification.
- [ ] Run `scripts/agent-validation.sh`.
- [ ] Commit, push, open one PR, and report its URL and validation evidence.

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
- The baseline row key is `(vendor, camera, format, path)`.
- Decoder stages are evaluated once each in order. Child-process timeout and
  panic handling turns decoder failures into baseline rows rather than
  infrastructure failures.
- The local full-run guard defaults to two files unless `CI=true` or
  `--allow-full` is present.
- Payload materialization happens before per-file timers start.
- The final workflow has only `workflow_dispatch`; a temporary branch push
  trigger exists solely for bootstrap review and is removed before handoff.
- The planned commit `174acd7...` was no longer present when implementation
  started. raw.pixls.us advertises one daily snapshot commit and replaces its
  history, so fresh checkouts follow the remote `master` branch with LFS
  smudging disabled. `manifest.csv` is the durable snapshot pin and
  `manifest --check` rejects uncommitted upstream changes.
