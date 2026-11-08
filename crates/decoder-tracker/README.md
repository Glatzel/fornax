# decoder-tracker

<!-- decoder-tracker-badge:start -->

[![RAW corpus: 209/2020 passed](https://img.shields.io/badge/raw.pixls.us_corpus_pass-209%2F2020-orange)](baseline.csv)

| Format | Status |
| --- | ---: |
| 3FR | 0/11 |
| ARI | 0/1 |
| ARQ | 0/1 |
| ARW | 0/206 |
| CAM | 0/2 |
| CR2 | 0/142 |
| CR3 | 0/110 |
| CRW | 0/51 |
| DCR | 0/5 |
| DNG | 209/262 |
| ERF | 0/3 |
| FFF | 0/11 |
| GPR | 0/17 |
| IIQ | 0/35 |
| KDC | 0/9 |
| LRI | 0/1 |
| MDC | 0/1 |
| MEF | 0/1 |
| MOS | 0/4 |
| MRW | 0/14 |
| NEF | 0/277 |
| NRW | 0/12 |
| ORF | 0/93 |
| ORI | 0/19 |
| PEF | 0/42 |
| RAF | 0/151 |
| RAW | 0/49 |
| RW2 | 0/402 |
| RWL | 0/20 |
| SR2 | 0/1 |
| SRF | 0/1 |
| SRW | 0/27 |
| STI | 0/1 |
| TIF | 0/20 |
| X3F | 0/18 |

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
`status` column makes pass/fail visually scannable. `update-badge.sh` joins the
full manifest to the baseline and writes the accumulated badge and per-format
`passed/total` table between the README markers; formats without baseline rows
report `0/total`. The complete schema, stage semantics, glob/sharding rules,
workflow, and update procedure are documented in
[docs/decoder/corpus.md](../../docs/decoder/corpus.md).
