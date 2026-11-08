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

Run from the repository root through the Pixi dev environment so `git-lfs` is
available:

```text
pixi run -e dev cargo run -q -p decoder-tracker -- manifest [--check] [--format F]...
pixi run -e dev cargo run -q -p decoder-tracker -- run [--manifest F] [--format F]...
    [--camera TEXT]... [--filename TEXT]... [--shard INDEX/COUNT]
    [--out FILE] [--max-files N] [--allow-full]
pixi run -e dev cargo run -q -p decoder-tracker -- merge [--manifest F] [--out FILE] SHARD...
pixi run -e dev cargo run -q -p decoder-tracker -- diff BASELINE CANDIDATE
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
are case-insensitive substring matches. Filters are applied before the
selection is sorted by LFS SHA-256 and assigned with zero-based
`position % COUNT`.

## Manifest

`manifest` reads `filelist.sha256`, selects supported formats, parses each
corresponding LFS pointer, verifies that its object ID equals the listed
SHA-256, derives identity fields, and writes the canonical CSV under
`crates/decoder-tracker/manifest.csv`.

The manifest header is:

```csv
vendor,camera,format,path,size,sha256
```

Repository paths use `/`, retain case and UTF-8 content, and are interpreted
relative to the corpus root. `camera` is normally `<vendor> <model>`; when the
model already starts with the vendor, the model is used without duplicating the
vendor. `filename` is derived as the original stem without its extension for
the `--filename` filter and is not duplicated in either committed CSV.

Manifest rows are sorted by `(sha256, path)`. `model` is derived from `path` when
the manifest is loaded rather than duplicated in the file. The unique baseline
key is `(vendor, camera, format, path)`. Duplicate keys are an error.

## Baseline schema

The canonical baseline header is:

```csv
vendor,camera,format,path,decode_pass,raw_mosaic_pass,process_metadata_pass,handoff_pass,error_kind,width,height,cfa
```

Rows use UTF-8 with LF endings and RFC 4180 quoting, sort by the unique key, and
store stage passes as `0` or `1`. There are no timestamps, timings, host data, or
floating-point values.

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

## Merge and diff

`merge` reads all shard CSVs, rejects schema mismatches and duplicate keys,
checks the complete expected key set from the manifest, and writes a canonical
full baseline. This is how the workflow prevents a missing shard from appearing
as an accepted baseline.

`diff` reports:

- `missing`: a baseline key absent from the candidate;
- `extra`: a candidate key absent from the baseline;
- `changed`: a key present in both with any field difference.

It exits 1 when any category is non-empty. Reordering rows is not a difference.

## Manual CI

`.github/workflows/decoder-tracker-corpus.yml` is manual-only after bootstrap.
It accepts:

- `mode`: `bootstrap` or `verify`;
- `shard_count`: an integer from 1 through 20, default 20.

The plan job generates a dynamic zero-based matrix. Each shard initializes the
pointer-only corpus, installs the Pixi dev environment, runs
`manifest --check`, runs its deterministic shard, and uploads CSV only. The
matrix limits concurrent jobs to five so simultaneous Pixi downloads do not
exhaust the installer endpoint. The aggregate job merges every shard:

- `bootstrap` uploads
  `target/decoder-tracker/candidate.csv` as a short-lived candidate artifact;
- `verify` diffs the merged candidate against
  `crates/decoder-tracker/baseline.csv` and requires an empty diff.

RAW payloads are never cached or uploaded.

## Corpus updates

A corpus update is deliberate:

1. Initialize the pointer-only submodule at the current advertised snapshot.
2. Run `manifest` and review every manifest change.
3. Run the manual workflow in `bootstrap` mode at 20 shards.
4. Download and review the complete candidate against the manifest.
5. Commit the manifest and baseline.
6. Run the manual workflow in `verify` mode and require an empty diff.

Transient full-mirror use is approved for this workflow. The baseline does not
redistribute RAW payloads. Some older corpus samples may retain non-CC0 licence
terms, so payloads remain in raw.pixls.us storage and are not published as
Fornax artifacts.

## Testing

`cargo test -p decoder-tracker` covers path normalization, identity derivation,
filters, deterministic sharding and bounds, stage transitions, metadata sanity,
child timeout and panic mapping, canonical CSV, merge completeness, diff
classification, and manifest pointer checks.

`scripts/agent-validation.sh` initializes only the corpus pointer metadata when
needed and runs `manifest --check`; it never fetches a RAW payload.
