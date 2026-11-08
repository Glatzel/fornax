# decoder-tracker

<!-- decoder-tracker-badge:start -->

[![raw.pixls.us corpus pass: 209/262 passed](https://img.shields.io/badge/DNG_corpus-209%2F262-orange)](baseline.csv)

| Format | Status |
| --- | ---: |
| DNG | 209/262 |

<!-- decoder-tracker-badge:end -->

Manual regression tracking for Fornax decoders over the pinned raw.pixls.us
corpus.

The corpus submodule is checked out as Git LFS pointer metadata. `run` fetches
only the selected shard's payloads, executes each file in an isolated child with
a 300-second timeout, and writes a canonical no-timings CSV. Full local runs
require `--allow-full` or `CI=true`; the default local guard permits two files.

```bash
pixi run -e dev cargo run -p decoder-tracker -- manifest --check
pixi run -e dev cargo run -p decoder-tracker -- select --glob 'Canon/**/*.DNG'
pixi run -e dev cargo run -p decoder-tracker -- run --filename IMG_0001
pixi run -e dev cargo run -p decoder-tracker -- merge --manifest MANIFEST SHARD...
pixi run -e dev cargo run -p decoder-tracker -- diff --selected MANIFEST BASELINE CANDIDATE
pixi run -e dev cargo run -p decoder-tracker -- update --baseline BASELINE --candidate CANDIDATE
pixi run -e tracker bash crates/decoder-tracker/scripts/update-badge.sh
```

The committed corpus metadata manifest is `manifest.csv`; decoder pass columns
and outcomes are in `baseline.csv`. DuckDB CLI owns plan-time manifest filtering
and the shell-driven merge, selected-key diff, and baseline upsert path. Stage
columns remain numeric for SQL; a leading emoji
`status` column makes pass/fail visually scannable. `update-badge.sh` writes the
accumulated badge and per-format `passed/total` table between the README markers.
The complete schema, stage semantics, glob/sharding rules, workflow, and update
procedure are documented in [docs/decoder/corpus.md](../../docs/decoder/corpus.md).
