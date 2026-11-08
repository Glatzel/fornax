# RAW corpus and regression tracking

## Status

`decoder-tracker` owns the manual raw.pixls.us decoder regression workflow. Its
first supported format is DNG. Each future native decoder adds its uppercase
format to the workspace's supported-format selection and bootstraps its rows
into the same text baseline.

The corpus checkout is `external/raw.pixls.us`, a Git LFS submodule backed by
`https://raw.pixls.us/data.lfs.git`. It is initialized with
`GIT_LFS_SKIP_SMUDGE=1`, so a normal checkout contains only LFS pointer metadata
and `filelist.sha256`. RAW payloads are fetched only for the selected shard and
are never committed, cached, or uploaded as artifacts.

raw.pixls.us replaces its Git LFS repository with a new daily snapshot instead
of retaining a permanent commit history. The submodule therefore follows the
remote's `master` branch for setup, while
`crates/decoder-tracker/manifest.csv` records the accepted snapshot's
paths, sizes, identities, and LFS SHA-256 values. `manifest --check` fails if the
current `filelist.sha256` or pointer set no longer matches that committed
manifest.

## Commands

Run from the repository root with globally installed `git-lfs` and through the
Pixi environments for DuckDB and decoder tooling:

```text
pixi run -e dev cargo run -q -p decoder-tracker -- manifest [--check] [--format F]...
pixi run -e dev cargo run -q -p decoder-tracker -- select [--glob G]... [--out FILE]
pixi run -e dev cargo run -q -p decoder-tracker -- run [--manifest F] [--format F]...
    [--camera TEXT]... [--filename TEXT]... [--glob G]... [--shard INDEX/COUNT]
    [--out FILE] [--max-files N] [--allow-full]
pixi run -e dev cargo run -q -p decoder-tracker -- merge --manifest F --out FILE SHARD...
pixi run -e dev cargo run -q -p decoder-tracker -- diff --selected F --out FILE BASELINE CANDIDATE
pixi run -e dev cargo run -q -p decoder-tracker -- update --baseline F --candidate F --out FILE
```

`run` defaults to DNG and refuses selections larger than two files unless
`CI=true` or `--allow-full` is present. Use camera and filename filters for
local investigation:

```bash
pixi run -e dev cargo run -q -p decoder-tracker -- run \
  --camera 'Canon EOS' --filename IMG_0001 \
  --out target/decoder-tracker/investigate.csv
```

Format and filename filters are case-insensitive exact matches. Camera filters
are case-insensitive substring matches. Path globs are case-sensitive:

- `*` matches within one path segment;
- `**` crosses directory separators;
- `?` matches one non-separator character;
- repeated `--glob` values use OR; format, camera, filename, and glob classes
  combine with AND.

The workflow default glob is `**/*.[dD][nN][gG]` because the corpus mixes
uppercase and lowercase DNG extensions. Filters are applied before the
selection is sorted by LFS SHA-256 and assigned with zero-based
`position % COUNT`.

## Manifest

`manifest` reads `filelist.sha256`, selects supported formats, parses each
corresponding LFS pointer, verifies that its object ID equals the listed
SHA-256, derives identity fields, and writes the canonical CSV under
`crates/decoder-tracker/manifest.csv`.

The manifest header is:

```csv
path,size,sha256
```

Repository paths use `/`, retain case and UTF-8 content, and are interpreted
relative to the corpus root. Camera, vendor, model, format, and filename are
derived from `path` when the manifest is loaded and are not duplicated in the
manifest CSV.

Manifest rows are sorted by `(sha256, path)`. `model` is derived from `path` when
the manifest is loaded rather than duplicated in the file. The unique baseline
key is `(camera, format, path)`. Duplicate keys are an error.

## Baseline schema

The canonical baseline header is:

```csv
status,camera,format,path,decode_pass,raw_mosaic_pass,process_metadata_pass,handoff_pass,error_kind,width,height,cfa
```

Rows use UTF-8 with LF endings and RFC 4180 quoting, sort by the unique key, and
store stage passes as `0` or `1`. There are no timestamps, timings, host data, or
floating-point values. The leading `status` column is a derived visual summary:
`✅` when all four stage passes are `1`, otherwise `❌`. Readers reject a status
that disagrees with the numeric stages.

The stages run once each in this order:

1. `decode_pass`: decoder construction succeeds.
2. `raw_mosaic_pass`: raw mosaic retrieval succeeds with non-zero dimensions
   and a matching sample count.
3. `process_metadata_pass`: DNG metadata and process metadata succeed, then
   geometry, active area, default crop, levels, and 2x2 CFA conversion pass
   sanity checks.
4. `handoff_pass`: `cropped_mosaic::<f32>` succeeds with the active-area
   dimensions and sample count.

Later stages are not attempted after an earlier failure. `error_kind` is empty
only when all four stages pass; otherwise it is `<stage>:<kind>`, such as
`raw_mosaic:bad_dimensions`, `process_metadata:unsupported`, or
`handoff:invalid_output`. Child panics, crashes, and timeouts are rows with
`runner:panic`, `runner:crash`, or `runner:timeout`.

The runner retains the best geometry and CFA label available after partial
failure. A non-2x2 CFA gets a descriptive label such as `XTrans-6x6` before the
typed unsupported CFA error is recorded.

## Isolation and LFS

`run` parses the manifest, selects rows, initializes Git LFS in the pointer-only
submodule, and fetches only the selected object paths before any decoder work.
Transfers use four bounded Git LFS workers, three retries, and explicit
diagnostics for rate-limit and network failures.

Each file is evaluated by a hidden `run-one` child process with a 300-second
timeout. Materialization time is outside that timeout. The parent converts
panics and crashes to rows, kills hangs, and treats malformed successful child
output, child launch failures, LFS failures, and I/O failures as infrastructure
errors. Decoder failures that were successfully recorded do not fail `run`.

## DuckDB aggregation

DuckDB CLI owns relational CSV aggregation. Rust validates input schemas and
typed rows, invokes checked-in SQL from `crates/decoder-tracker/sql/`, and then
revalidates the canonical output.

`merge` combines shard CSVs, rejects duplicates, and verifies that every key in
the selected manifest appears exactly once. `diff --selected` compares only the
selected keys against the full committed baseline. It writes a canonical diff
with:

- `missing`: a baseline key absent from the candidate;
- `extra`: a candidate key absent from the baseline;
- `changed`: a key present in both with any field difference.

`update` upserts candidate rows into the full baseline and preserves unselected
rows. The diff command exits 1 when any category is non-empty.

## Manual CI

`.github/workflows/decoder-tracker-corpus.yml` is manual-only after bootstrap.
It accepts one `filter` path glob and an optional `run_duckdb_tests` boolean
(default `false`). The plan job writes and uploads `selected-manifest.csv`, then
computes:

```text
runner_count = min(20, max(1, ceil(selected_count / 10)))
```

Rows are balanced across those runners by `position % runner_count`. Examples:
5 files use one runner, 16 use two runners of 8, and 210 use 20 runners of
10/11. Zero matches fail planning.

Each shard downloads the selected manifest, initializes the pointer-only corpus,
runs `manifest --check`, executes its assigned files, and uploads CSV only. The
aggregate job merges every shard and:

- uploads a bootstrap baseline when no committed baseline exists;
- uploads nothing when selected rows are unchanged;
- uploads the complete updated baseline and `diff.csv`, then fails, when rows
  changed.

RAW payloads are never cached or uploaded.

## Corpus updates

A corpus update is deliberate:

1. Initialize the pointer-only submodule at the current advertised snapshot.
2. Run `manifest` and review every manifest change.
3. Run the manual workflow with the full default DNG glob.
4. Download and review the complete candidate against the manifest.
5. Commit the manifest and baseline.
6. Run the workflow again and require no update artifact.

Transient full-mirror use is approved for this workflow. The baseline does not
redistribute RAW payloads. Some older corpus samples may retain non-CC0 licence
terms, so payloads remain in raw.pixls.us storage and are not published as
Fornax artifacts.

## Testing

`cargo test -p decoder-tracker` covers path normalization, identity derivation,
glob and legacy filters, adaptive runner sizing, deterministic sharding and
bounds, stage transitions, metadata sanity, child timeout and panic mapping,
canonical CSV, DuckDB merge completeness, DuckDB diff classification, partial
upsert, and manifest pointer checks.

`scripts/agent-validation.sh` initializes only the corpus pointer metadata when
needed, runs `manifest --check`, and runs the DuckDB aggregation tests through
the tracker environment; it never fetches a RAW payload.
