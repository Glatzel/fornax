---
name: decoder-corpus-tracker
description: Use when generating or verifying the raw.pixls.us corpus manifest, running selected decoder corpus cases, merging shard CSVs, refreshing the decoder baseline, or regenerating the decoder-tracker README.
---

# Maintain decoder corpus tracking

Read [crates/decoder-tracker/AGENTS.md](../../../crates/decoder-tracker/AGENTS.md)
before acting. Follow its setup, command, baseline, and README rules. Read
[docs/decoder/corpus.md](../../../docs/decoder/corpus.md) for schemas, stage
semantics, and CI internals.

Choose one route:

| Situation | Route |
| --- | --- |
| Inspect one camera, file, format, or path | [Local investigation](#local-investigation) |
| Run an existing selection across local shards | [Local shard merge](#local-shard-merge) |
| Decoder behavior changed | [Manual baseline update](#manual-baseline-update) |
| raw.pixls.us snapshot changed | [Corpus snapshot update](#corpus-snapshot-update) |
| Only the badge/table is stale | [Refresh the README](#refresh-the-readme) |

Keep RAW payloads transient. Never commit, cache, or upload them. Never
hand-edit `manifest.csv`, `baseline.csv`, or the generated README marker
region.

## Prepare the pointer-only corpus

Preserve the committed submodule revision:

```bash
GIT_LFS_SKIP_SMUDGE=1 git -c filter.lfs.required=false \
  submodule update --init external/raw.pixls.us
pixi run -e dev cargo run -q -p decoder-tracker -- manifest --check
```

Use `--remote` only for an intentional snapshot update. Stop if
`manifest --check` disagrees with the committed manifest during routine work.

## Local investigation

`run` materializes only the selection and permits at most two files unless
`--allow-full` or `CI=true` is present:

```bash
pixi run -e dev cargo run -q -p decoder-tracker -- run \
  --camera 'Canon EOS' --filename IMG_0001 \
  --out target/decoder-tracker/investigate.csv
```

Format and filename filters are case-insensitive exact matches. Camera filters
are case-insensitive substrings. Path globs are case-sensitive.

## Local shard merge

Create one selected manifest, run every shard, and merge against that same
manifest:

```bash
pixi run -e dev cargo run -q -p decoder-tracker -- select \
  --glob '**/*.[dD][nN][gG]' \
  --out target/decoder-tracker/selected-manifest.csv

# Repeat for 0/COUNT .. (COUNT-1)/COUNT.
pixi run -e dev cargo run -q -p decoder-tracker -- run \
  --manifest target/decoder-tracker/selected-manifest.csv \
  --shard 0/COUNT --allow-full \
  --out target/decoder-tracker/shards/shard-0.csv

pixi run -e dev cargo run -q -p decoder-tracker -- merge \
  --manifest target/decoder-tracker/selected-manifest.csv \
  --out target/decoder-tracker/candidate.csv \
  target/decoder-tracker/shards/*.csv
```

Merge errors are a stop condition: selected keys are missing, duplicated, or
unexpected. Do not replace the committed baseline with a partial candidate.

## Manual baseline update

The workflow advances the pointer-only submodule to current raw.pixls.us
`master` before checking the committed manifest. If it fails on a manifest
mismatch, use the corpus snapshot update route first; do not accept baseline
rows from a run whose corpus revision does not match the committed manifest.

Commit and push the decoder change first. Before dispatching, confirm the
remote branch is the revision to test because the workflow cannot see local
uncommitted commits or files. Run the manual workflow from that branch:

```bash
gh workflow run decoder-tracker-corpus.yml --ref <branch> \
  -f filter='**/*.[dD][nN][gG]' \
  -f run_duckdb_tests=false
gh run list --workflow decoder-tracker-corpus.yml --branch <branch>
gh run view <run-id>
```

If the run is unchanged, no artifact is expected and no baseline change is
needed. If decoder rows changed, the aggregate job intentionally fails and
uploads `decoder-baseline-update`:

```bash
gh run download <run-id> --name decoder-baseline-update \
  --dir target/decoder-tracker/update
```

1. Review `diff.csv` and verify every changed key is intended.
2. Review `candidate.csv` and the complete `baseline.csv`.
3. Copy only the complete reviewed baseline:

   ```bash
   cp target/decoder-tracker/update/baseline.csv \
     crates/decoder-tracker/baseline.csv
   ```

4. Refresh the README as below.
5. Inspect `git diff`.
6. Commit and push the updated branch so the remote workflow can see the
   reviewed files.
7. Rerun the workflow and require `unchanged` with no update artifact.

Rerun in place with `gh run rerun <run-id> --failed` only for a transient
infrastructure failure. A changed-row failure requires review, not a blind
retry.

## Corpus snapshot update

Only this route advances the submodule:

```bash
GIT_LFS_SKIP_SMUDGE=1 git -c filter.lfs.required=false \
  -c submodule.external/raw.pixls.us.branch=master \
  submodule update --init --remote external/raw.pixls.us
pixi run -e dev cargo run -q -p decoder-tracker -- manifest
```

Review every `manifest.csv` change, run the full DNG workflow, apply its
reviewed artifact, then commit the submodule pointer
(`external/raw.pixls.us`), manifest, baseline, and README together. Push the
updated branch before rerunning the workflow and require an unchanged outcome.

## Refresh the README

After accepting the baseline:

```bash
pixi run -e tracker bash crates/decoder-tracker/scripts/update-badge.sh
```

The script joins the full manifest to the baseline and replaces only the
badge/table region. Review the diff. Never edit between
`decoder-tracker-badge:start` and `decoder-tracker-badge:end` by hand.

## Check the change

- `diff --selected` reports only the reviewed selection.
- `update` preserves unselected baseline rows.
- The README script preserves text outside its markers.
- `manifest.csv`, `baseline.csv`, and generated README data contain no RAW
  payload data.
- Run `cargo test -p decoder-tracker`, the DuckDB tests, and repository
  validation required by the root guide.
- Run `git diff --check` and, for documentation-only changes,
  `scripts/agent-validation.sh --docs-only`.
