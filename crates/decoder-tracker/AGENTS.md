# decoder-tracker: maintainer guide

Tracks Fornax decoder results over the pinned
[raw.pixls.us corpus](../../docs/reference/decoders/operations/corpus.md). The manifest and baseline
are canonical text data. RAW payloads are transient. Repository-wide workflow
lives in the [root agent guide](../../AGENTS.md); this file covers only corpus
tracking.

Use the [decoder-corpus-tracker skill](../../.agents/skills/decoder-corpus-tracker/SKILL.md)
for task procedures. Read
[docs/reference/decoders/operations/corpus.md](../../docs/reference/decoders/operations/corpus.md)
for the CSV schemas, stage semantics, sharding rules, and workflow internals.

## Commands

Run from the repository root:

```text
decoder-tracker manifest [--check]
decoder-tracker select [--camera C]... [--filename N]...
    [--glob G]... [--manifest F] [--out F]
decoder-tracker run [filters] [--manifest F] [--shard I/N] [--out F]
    [--max-files N] [--allow-full]
tracker-data merge [--manifest F] [--out F] SHARD...
tracker-data diff --selected F [--out F] BASELINE CANDIDATE
tracker-data update [--baseline F] --candidate F [--out F]
decoder-tracker hash [--manifest F] [--out F | --check F]
decoder-tracker golden-coverage [--baseline F] [--corpus-manifest F]
    [--golden F] [--out F] [--selected-out F]
```

Use `pixi run -e tracker decoder-tracker <subcommand>` for Rust tracker
commands, `tracker-data` for CSV operations, and `tracker-badge` for the README.

Common checks:

```bash
pixi run -e tracker decoder-tracker manifest --check
pixi run -e tracker decoder-tracker run --filename IMG_0001
pixi run -e tracker decoder-tracker-release hash --check \
  crates/decoder-tracker/golden/decode-hashes.csv
pixi run -e tracker decoder-tracker golden-coverage
pixi run -e tracker tracker-badge
```

`run` fetches only selected LFS payloads and defaults to at most two files.
Use filters or a shard for investigation. A deliberate larger local run needs
`--allow-full`; manual CI sets `CI=true`. Full runs are manual-only.

## Initialize the corpus

Routine work must use the submodule commit already recorded by Fornax:

```bash
GIT_LFS_SKIP_SMUDGE=1 git -c filter.lfs.required=false \
  submodule update --init external/raw.pixls.us
```

This leaves the checkout as pointer metadata. Do not fetch all RAW payloads.

Only a deliberate corpus snapshot refresh advances the submodule to
raw.pixls.us `master`:

```bash
GIT_LFS_SKIP_SMUDGE=1 git -c filter.lfs.required=false \
  -c submodule.external/raw.pixls.us.branch=master \
  submodule update --init --remote external/raw.pixls.us
```

That command changes the submodule gitlink and requires the manifest and
baseline refresh workflow below. A routine decoder change must not use it.

After either form of initialization, require:

```bash
pixi run -e tracker decoder-tracker manifest --check
```

Use root `scripts/setup.sh` only for `external/raw-images`; it does not
initialize the raw.pixls.us corpus.

## Local shard merge

`select` writes the selected-key manifest. `run --shard I/N` splits that
selection deterministically. `merge` requires every selected key exactly once,
then `diff --selected` compares only those keys with the committed baseline:

```bash
pixi run -e tracker decoder-tracker select \
  --glob '*/**/*' \
  --out target/decoder-tracker/selected-manifest.csv
pixi run -e tracker decoder-tracker run \
  --manifest target/decoder-tracker/selected-manifest.csv \
  --shard 0/2 --allow-full \
  --out target/decoder-tracker/shards/shard-0.csv
pixi run -e tracker tracker-data merge \
  --manifest target/decoder-tracker/selected-manifest.csv \
  --out target/decoder-tracker/candidate.csv \
  target/decoder-tracker/shards/*.csv
pixi run -e tracker tracker-data diff \
  --selected target/decoder-tracker/selected-manifest.csv \
  crates/decoder-tracker/data/baseline.csv \
  target/decoder-tracker/candidate.csv
```

`merge` rejects missing, duplicate, unexpected, and schema-invalid rows. A
partial candidate is never suitable as the committed baseline.

## Manual baseline update

Prefer the manual sharded workflow for a full refresh:

```bash
gh workflow run decoder-tracker-corpus.yml --ref <branch> \
  -f filter='*/**/*' \
  -f run_duckdb_tests=false
gh run list --workflow decoder-tracker-corpus.yml --branch <branch>
gh run view <run-id>
```

The workflow advances the pointer-only submodule to the current
raw.pixls.us `master` before checking the committed manifest. A decoder-result
update therefore requires the committed manifest to match that remote
snapshot. If the workflow fails with a manifest mismatch, follow the corpus
snapshot update path before accepting baseline rows.

Commit and push the decoder change first. Before dispatching, confirm the
remote branch is the revision to test because the workflow cannot see local
uncommitted commits or files.

An unchanged result uploads no update artifact. A changed result uploads
`decoder-baseline-update`, then the aggregate job intentionally fails so a
person reviews the change:

```bash
gh run download <run-id> --name decoder-baseline-update \
  --dir target/decoder-tracker/update
```

Review `diff.csv`, `candidate.csv`, and `baseline.csv`. Apply the complete
reviewed artifact baseline, never the partial candidate:

```bash
cp target/decoder-tracker/update/baseline.csv \
  crates/decoder-tracker/data/baseline.csv
pixi run -e tracker tracker-badge
```

Inspect `git diff`, then commit and push the updated branch. Rerun the same
workflow only after the remote branch contains the reviewed baseline and
README. It must report unchanged and upload no update artifact before the
baseline is considered accepted.

Source encoding metadata is part of each baseline row. Compression, storage,
bit depth, sample format, predictor, or geometry changes therefore appear in
`decoder-baseline-update/diff.csv` and the complete `baseline.csv`, and require
the same review as stage or output changes.

## Corpus snapshot update

1. Advance the pointer-only submodule to the advertised snapshot.
2. Run `manifest` and review every change to `manifest.csv`.
3. Run the full default `*/**/*` workflow and apply its changed artifacts as
   above.
4. Commit the submodule pointer (`external/raw.pixls.us`), manifest, complete
   baseline, and regenerated README together.
5. Rerun the workflow and require an unchanged outcome.

Follow the deliberately bounded procedure in
[docs/reference/decoders/operations/corpus.md](../../docs/reference/decoders/operations/corpus.md#corpus-updates).

## Files and invariants

- `data/manifest.csv` identifies every corpus path, size, and LFS SHA-256.
- `data/baseline.csv` is the accepted full result set, including source
  encoding metadata for every successfully decoded row. Do not hand-edit it.
- Current golden test/perf manifests, manual selection rules, and hash policy
  are documented in `golden/README.md`.
- `golden/decode-hashes.csv` contains SHA-256 values for raw samples, cropped
  samples, process metadata, and DNG metadata. Generate it with `hash --out`;
  never hand-edit it.
- `README.md` owns the badge and vendor-format table only between
  `decoder-tracker-badge` markers. Regenerate that region with
  `update-badge.sh`; never edit it manually.
- `diff` and `update` are selected-key operations. `update` preserves every
  baseline row outside the reviewed selection.
- RAW payloads are never committed, cached, or uploaded as artifacts.
- Failed rows are results, not runner failures. Distinguish typed stage errors
  from `runner:*` status before changing the baseline.
- Golden cases must be known passing rows. The test set may include an
  explicit lossy regression after a full conventional-Bayer handoff, while the
  performance set remains lossless-only. A failed decode is not a valid golden
  case and must not cause partial output.
- `golden-coverage` reports lossless decoder paths absent from golden and
  golden groups that duplicate the same decoder path. DNG uses codec-kernel
  groups while retaining LJPEG strip/tile layout; non-DNG formats retain their
  encoding key. CFA is excluded, and the representative is the lowest-pixel
  passing file.
  `--selected-out` writes one representative per decoder path but never
  replaces the golden manifest implicitly.

## Validate tracker changes

For tracker code or guidance changes, run the task-specific tests:

```bash
pixi run -e tracker tracker-test
```

Run the repository validation required by the root guide. Documentation-only
changes use `scripts/agent-validation.sh --docs-only`.
