# Decoder Tracker Python and DuckDB Consolidation Plan

## Goal

Remove duplicated tabular mechanics from `decoder-tracker` by making one Python
entry point and DuckDB SQL the source of truth for selecting, sharding,
merging, diffing, and updating CSV data. Keep decoder execution, hashing,
manifest validation, and portable process isolation in Rust.

## Scope

In scope:

- Replace separate Rust and shell implementations of merge, diff, and upsert.
- Move path/camera/filename filtering and deterministic sharding into Python.
- Remove the `duckdb` and `globset` dependencies where possible.
- Keep a small Rust boundary for decoder execution, manifest generation,
  golden hashing, and portable child timeouts.
- Update tracker commands, tests, and maintainer documentation.

Out of scope:

- Moving decoder calls, metadata sanity checks, or hash serialization to SQL.
- Rewriting historical plans or generated baseline/manifest data.
- Changing decoder support, corpus results, or golden expectations.

## Design

Add `scripts/tracker_data.py` using Typer, `pathspec`, and the Python DuckDB
binding. Python owns argument parsing and glob filtering so selection behavior
can be extended without constructing SQL predicates or shell-quoted filters.

```text
select  --manifest F --out F [--camera C]... [--filename N]... [--glob G]...
        [--shard I/N]
merge   --manifest F --out F SHARD...
diff    --selected F --out F BASELINE CANDIDATE
update  --baseline F --candidate F --out F
```

The Python command applies case-insensitive camera substrings, case-insensitive
exact filenames, and segment-aware path globs directly to manifest rows. It then
sorts by `(sha256, path)` and performs zero-based modulo shard assignment.
DuckDB executes the existing relational merge, diff, and upsert templates.

`aggregate.sh` will call the same Python data command instead of carrying
duplicate SQL invocations. Rust command dispatch retains `manifest`, `run`, `hash`,
`golden-coverage`, `golden-check`, and hidden `run-one`; the Rust `select`
command delegates to the data script. `run` delegates selection to the script,
then retains materialization and child isolation.

## Task Checklist

- [x] Add Python tests for selection and data operations.
- [x] Implement `tracker_data.py select` and Python selection/sharding.
- [x] Implement `tracker_data.py merge`, `diff`, and `update` using existing SQL.
- [x] Refactor `aggregate.sh` to call the shared Python command.
- [x] Replace Rust selection logic with the shared Python command.
- [x] Remove Rust merge/diff/update commands and `duckdb.rs`.
- [x] Remove `globset` and obsolete `select.sh`.
- [x] Update tracker README, AGENTS, and corpus operations documentation.
- [ ] Run tracker, DuckDB, and repository validation.

## Files Expected To Change

- `crates/decoder-tracker/scripts/tracker_data.py`
- `crates/decoder-tracker/scripts/test_tracker_data.py`
- `crates/decoder-tracker/scripts/aggregate.sh`
- `crates/decoder-tracker/scripts/select.sh` (delete)
- `crates/decoder-tracker/src/selection.rs`
- `crates/decoder-tracker/src/main.rs`
- `crates/decoder-tracker/src/lib.rs`
- `crates/decoder-tracker/src/duckdb.rs` (delete)
- `crates/decoder-tracker/Cargo.toml`
- `crates/decoder-tracker/tests/`
- `crates/decoder-tracker/README.md`
- `crates/decoder-tracker/AGENTS.md`
- `docs/reference/decoders/operations/corpus.md`
- `pixi.toml`
- `.github/workflows/decoder-tracker-corpus.yml`

## Validation

```bash
cargo test -p decoder-tracker
pixi run -e tracker tracker-test
scripts/agent-validation.sh
```

## Decisions And Notes

- SQL validation remains responsible for relational invariants shared by merge,
  diff, and update.
- Rust remains the trusted boundary for pointer parsing, decoding, hashing, and
  process isolation.
- The existing SQL files remain canonical and are rendered by the Python command.
- `run` writes a temporary selected manifest and removes it after the run.
- Typer, `pathspec`, and `python-duckdb` are development dependencies of the
  DuckDB-enabled Pixi environment.
