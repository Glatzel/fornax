# RAW Corpus and Regression Tracking

This guide documents the pinned RAW corpus, golden regression workflow, schemas, and manual CI operation.

## Status

`decoder-tracker` owns the manual raw.pixls.us decoder regression workflow. The
manifest records the complete pinned corpus independently of decoder support or
baseline coverage. Its first supported format is DNG. Each future native
decoder adds its uppercase format to the run selection and bootstraps its rows
into the same text baseline.

The corpus checkout is `external/raw.pixls.us`, a Git LFS submodule backed by
`https://raw.pixls.us/data.lfs.git`. It is initialized with
`GIT_LFS_SKIP_SMUDGE=1`, so a normal checkout contains only LFS pointer metadata
and `filelist.sha256`. RAW payloads are fetched only for the selected shard and
are never committed, cached, or uploaded as artifacts.

raw.pixls.us replaces its Git LFS repository with a new daily snapshot instead
of retaining a permanent commit history. The submodule therefore follows the
remote's `master` branch for setup, while
`crates/decoder-tracker/data/manifest.csv` records the accepted snapshot's
paths, sizes, identities, and LFS SHA-256 values. `manifest --check` fails if the
current `filelist.sha256` or pointer set no longer matches that committed
manifest.

## Commands

Run from the repository root with globally installed `git-lfs` and through the
Pixi environments for DuckDB and decoder tooling:

```text
pixi run -e dev cargo run -q -p decoder-tracker -- manifest [--check]
pixi run -e dev cargo run -q -p decoder-tracker -- select [--glob G]... [--out FILE]
pixi run -e dev cargo run -q -p decoder-tracker -- run [--manifest F]
    [--camera TEXT]... [--filename TEXT]... [--glob G]... [--shard INDEX/COUNT]
    [--out FILE] [--max-files N] [--allow-full]
pixi run -e dev cargo run -q -p decoder-tracker -- merge --manifest F --out FILE SHARD...
pixi run -e dev cargo run -q -p decoder-tracker -- diff --selected F --out FILE BASELINE CANDIDATE
pixi run -e dev cargo run -q -p decoder-tracker -- update --baseline F --candidate F --out FILE
pixi run -e dev cargo run -q -p decoder-tracker -- hash [--manifest F] [--out FILE | --check FILE]
pixi run -e dev cargo run -q -p decoder-tracker -- corpus-metadata --manifest F --shard I/N --out FILE
pixi run -e dev cargo run -q -p decoder-tracker -- corpus-metadata-merge --manifest F --out FILE SHARD...
pixi run -e tracker bash crates/decoder-tracker/scripts/update-badge.sh
```

`run` does not apply an implicit format filter. With the default manifest and
no camera, filename, or glob filters it selects the complete corpus, then
refuses selections larger than two files unless `CI=true` or `--allow-full` is
present. Use camera and filename filters for local investigation:

```bash
pixi run -e dev cargo run -q -p decoder-tracker -- run \
  --camera 'Canon EOS' --filename IMG_0001 \
  --out target/decoder-tracker/investigate.csv
```

Filename filters are case-insensitive exact matches. Camera filters are
case-insensitive substring matches. Path globs are case-sensitive:

- `*` matches within one path segment;
- `**` crosses directory separators;
- `?` matches one non-separator character;
- repeated `--glob` values use OR; camera, filename, and glob classes combine
  with AND.

Format is derived from the file extension but is not a separate command-line
filter. Select one format with an extension glob, such as
`**/*.[dD][nN][gG]` for DNG.

The workflow default glob is `*/**/*`, which selects the complete corpus across
all supported and unsupported formats. Filters are applied before the selection
is sorted by LFS SHA-256 and assigned with zero-based `position % COUNT`.

## Manifest

`manifest` reads `filelist.sha256`, parses every corresponding LFS pointer,
verifies that its object ID equals the listed SHA-256, derives identity fields,
and writes the canonical CSV under
`crates/decoder-tracker/data/manifest.csv`. The
manifest always covers every format in the corpus, including formats without a
decoder implementation or baseline rows.

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
status,camera,format,path,decode_pass,raw_mosaic_pass,raw_sha256,process_metadata_pass,handoff_pass,error_kind,width,height,cfa
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
   `raw_sha256` records the domain-separated storage-native raw mosaic hash
   when this stage passes; legacy rows may have an empty value until refreshed.
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

## DNG corpus metadata schema

`corpus-metadata.csv` is a generated sidecar used to analyze benchmark and
coverage groups by the actual DNG container tags rather than filenames. It does
not change the pointer-only manifest or the required baseline schema. DNG is
the first supported format; future formats can add their own rows without
changing the manifest. The `corpus-metadata` command internally selects rows
whose derived format is DNG; this format-specific behavior is not exposed as a
generic CLI filter.

The canonical header is:

```csv
status,path,sha256,format,compression_id,compression_name,storage,bits_per_sample,sample_format,predictor,rows_per_strip,tile_width,tile_height,error_kind
```

Rows are keyed by `(sha256, path)` and cover every selected DNG exactly once.
For a parseable DNG, `status` is `✅`, `storage` is `strips` or `tiles`, and
absent layout fields and the default TIFF predictor of `1` are explicit so
grouping does not need to reinterpret missing values. The classifier uses the
full-resolution raw IFD selected by `fornax-dng`, including DNG-owned
compression identifiers `34892` and `52546`. It records unknown IDs rather than
inferring compression from the path.

A DNG that cannot be parsed or classified is a failed result rather than a
workflow error. It gets `status` `❌`, `compression_name` and `storage` of
`unknown`, empty classification values, and a typed `error_kind` such as
`metadata:invalid_input` or `metadata:unsupported`. File I/O, LFS, and other
infrastructure failures remain fatal.

Generate a local selection with `select`, then classify all of it:

```bash
pixi run -e dev cargo run -q -p decoder-tracker -- corpus-metadata \
  --manifest target/decoder-tracker/selected-metadata.csv \
  --allow-full \
  --out target/decoder-tracker/corpus-metadata.csv
```

For sharded use, pass the same selection and shard to each runner, then merge
all metadata shards:

```bash
pixi run -e dev cargo run -q -p decoder-tracker -- corpus-metadata \
  --manifest target/decoder-tracker/selected-manifest.csv \
  --shard 0/COUNT --allow-full \
  --out target/decoder-tracker/metadata-shards/shard-0.csv
pixi run -e dev cargo run -q -p decoder-tracker -- corpus-metadata-merge \
  --manifest target/decoder-tracker/selected-manifest.csv \
  --out target/decoder-tracker/corpus-metadata.csv \
  target/decoder-tracker/metadata-shards/*.csv
```

The merge rejects missing, duplicate, unexpected, or malformed DNG rows,
including failed results that carry contradictory classification fields. The
committed sidecar is generated data and must not be hand-edited.

## Golden decode outputs

`golden/selected-manifest.csv` is a pointer-only set of 30 known-passing
conventional 2x2 RGB Bayer files. It covers DNG integer and floating-point
storage, CR3, ARW, NEF, RAF, both accepted 3FR compression families, RW2, and
IIQ. `golden/decode-hashes.csv` is the canonical output gate for refactors that
must not change decoder results.

Each row records the path, raw sample type and dimensions, cropped `f32`
dimensions, and four SHA-256 digests:

- `raw_sha256` covers the storage-native samples, sample type, and dimensions;
- `cropped_sha256` covers active-area cropped `f32` sample bits;
- `process_sha256` covers geometry, levels, Bayer pattern, and resolved color;
- `dng_sha256` covers the normalized TIFF/DNG-style metadata directory.

Write or verify the golden set with:

```bash
pixi run -e dev cargo run --release -q -p decoder-tracker -- hash \
  --manifest crates/decoder-tracker/golden/selected-manifest.csv \
  --out crates/decoder-tracker/golden/decode-hashes.csv
pixi run -e dev cargo run --release -q -p decoder-tracker -- hash \
  --manifest crates/decoder-tracker/golden/selected-manifest.csv \
  --check crates/decoder-tracker/golden/decode-hashes.csv
```

Golden corpus hashing must use the release profile; debug-mode decodes are too
slow for routine local verification and CI. `hash` materializes only the
selected LFS payloads. A failed decode aborts without writing a partial CSV.
The golden manifest and hashes are generated data and must not be hand-edited.

## Reading failures

The baseline records outcomes, not decoder internals. A failed row is not
automatically evidence of a missing compression codec:

- `raw_mosaic:unsupported` currently includes three-sample `LinearRaw` DNGs
  that cannot enter the mono `RawMosaic` contract.
- `raw_mosaic:invalid_input` includes unsupported lossless-JPEG tile variants,
  missing TIFF layout tags such as `RowsPerStrip`, and truncated or mismatched
  strip/tile chunks.
- `process_metadata:unsupported` includes CFA layouts outside the conventional
  2x2 RGB Bayer handoff, such as X-Trans.
- `process_metadata:backend` and `decode:invalid_input` must be investigated
  from their source sample before being assigned to a codec gap.

Compression `52546` is the DNG 1.7 JPEG XL identifier and `50002` is the
self-assigned TIFF JPEG XL identifier. Both use the JPEG XL decoder. Zstandard
uses pure-Rust `structured-zstd` when the `zstd` feature is enabled.

`update-badge.sh` joins the full manifest to the baseline and writes one badge
plus a per-vendor-format table into the decoder-tracker README. Vendor is the
first path segment, such as `Canon` or `Sony`. The table includes every
vendor-format pair in the manifest; pairs without baseline rows report
`0/total`. The badge always reports the accumulated manifest total. Its slug is
`raw.pixls.us_corpus_pass`, which renders the label `raw.pixls.us corpus pass`.

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
(default `false`). The default `*/**/*` filter selects the complete corpus; a
narrower filter selects a DNG, ARW, or mixed subset. The selected manifest
determines the formats processed by each shard. The plan job uses
`crates/decoder-tracker/scripts/select.sh` to filter `manifest.csv` directly with
DuckDB, writes and uploads `selected-manifest.csv`, then computes:

```text
runner_count = min(20, max(1, ceil(selected_count / 10)))
```

A separate build job compiles `decoder-tracker` once in release mode and
uploads the binary. Shards download that artifact and execute it directly, so
they do not compile the workspace independently.

Rows are balanced across those runners by `position % runner_count`. Examples:
5 files use one runner, 16 use two runners of 8, and 210 use 20 runners of
10/11. Zero matches fail planning.

Workflow concurrency is scoped to the branch and cancels superseded runs. Failed
jobs should be retried in place with `gh run rerun --failed` after transient
GitHub/Pixi download failures.

Each shard downloads the selected manifest and release tracker binary,
initializes the pointer-only corpus, runs `manifest --check`, executes its
assigned decoder rows, classifies assigned DNG compression metadata, and
uploads CSV only. The aggregate job uses
`crates/decoder-tracker/scripts/aggregate.sh` to merge, validate, diff, and
upsert decoder rows entirely through DuckDB. It separately merges and compares
the DNG metadata sidecar, then:

- uploads a bootstrap baseline when no committed baseline exists;
- uploads nothing when selected rows are unchanged;
- uploads the complete updated baseline and `diff.csv`, then fails, when rows
  changed.
- uploads `decoder-corpus-metadata-update` and fails when the DNG metadata
  sidecar changed.

Malformed corpus DNGs are represented in the metadata sidecar as `❌` failed
rows; they do not fail the shard. The intentional changed-artifact review gate
still fails the aggregate until the generated sidecar is reviewed and
committed.

RAW payloads are never cached or uploaded.

## Running the manual workflow

Run the workflow from the branch under review, then use the printed run ID for
the review commands:

```bash
gh workflow run decoder-tracker-corpus.yml --ref <branch> \
  -f filter='*/**/*' \
  -f run_duckdb_tests=false
gh run list --workflow decoder-tracker-corpus.yml --branch <branch>
gh run view <run-id>
```

Each shard advances the pointer-only submodule to current raw.pixls.us `master`
before running `manifest --check`. If the check reports a manifest mismatch,
use the corpus snapshot update procedure below before applying any baseline
rows from the run.

Commit and push the decoder change first. Before dispatching, confirm the
remote branch is the revision to test because the workflow cannot see local
uncommitted commits or files.

Treat `gh run rerun <run-id> --failed` as an infrastructure retry only. A
changed-row failure is the review gate described below.

## Applying a manual workflow result

An unchanged run uploads no update artifact. A decoder change intentionally
fails after uploading `decoder-baseline-update` with three CSV files:

- `diff.csv` classifies selected keys as missing, extra, or changed;
- `candidate.csv` contains the merged selected rows;
- `baseline.csv` is the complete baseline after the selected-key upsert.

A DNG metadata change independently uploads
`decoder-corpus-metadata-update/corpus-metadata.csv` and fails the run. Review
the complete sidecar, copy it to
`crates/decoder-tracker/data/corpus-metadata.csv`,
commit it, and rerun until it is unchanged. Do not merge a partial candidate.

Download and inspect the artifact:

```bash
gh run download <run-id> --name decoder-baseline-update \
  --dir target/decoder-tracker/update
```

Review every entry in `diff.csv` against the decoder change. Verify that
`candidate.csv` has no unexplained additions, and review the upserted rows in
the complete `baseline.csv`. Copy only the complete reviewed baseline; a
candidate from a partial selection must never replace the committed baseline:

```bash
cp target/decoder-tracker/update/baseline.csv \
  crates/decoder-tracker/data/baseline.csv
pixi run -e tracker bash crates/decoder-tracker/scripts/update-badge.sh
```

`update-badge.sh` owns the regions between the decoder-tracker badge markers.
Review the README diff and never edit those regions by hand.

For a corpus snapshot update, commit the reviewed `manifest.csv` with the full
baseline and regenerated README. Commit only the baseline and README for a
decoder-result update with an unchanged manifest.

Finally, rerun the same workflow from the updated branch. Require an
`unchanged` outcome and no `decoder-baseline-update` artifact before accepting
the baseline. Commit and push the reviewed baseline and README first, because
`gh workflow run --ref <branch>` tests the remote branch rather than local
uncommitted files.

## Corpus updates

A corpus update is deliberate:

1. Initialize the pointer-only submodule at the current advertised snapshot.
2. Run `manifest` and review every manifest change.
3. Run the manual workflow with the full default
   `*/**/*` glob.
4. Download and review the complete candidate against the manifest.
5. Commit the submodule pointer (`external/raw.pixls.us`), manifest, complete
   baseline, and regenerated README.
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
upsert, canonical golden hashes, golden diff reporting, and manifest pointer
checks.

`scripts/agent-validation.sh` initializes only the corpus pointer metadata when
needed, runs `manifest --check`, and runs the DuckDB aggregation tests through
the tracker environment; it never fetches a RAW payload.
