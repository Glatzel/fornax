# decoder-tracker

Manual regression tracking for Fornax decoders over the pinned raw.pixls.us
corpus.

The corpus submodule is checked out as Git LFS pointer metadata. `run` fetches
only the selected shard's payloads, executes each file in an isolated child with
a 300-second timeout, and writes a canonical no-timings CSV. Full local runs
require `--allow-full` or `CI=true`; the default local guard permits two files.

```bash
pixi run -e dev cargo run -p decoder-tracker -- manifest --check
pixi run -e dev cargo run -p decoder-tracker -- run --filename IMG_0001
pixi run -e dev cargo run -p decoder-tracker -- merge SHARD...
pixi run -e dev cargo run -p decoder-tracker -- diff BASELINE CANDIDATE
```

The committed corpus metadata manifest is `baseline.csv`; decoder pass columns
and outcomes are in `baseline/corpus.csv`. The complete schema, stage semantics,
sharding rules, workflow, and update procedure are documented in
[docs/decoder/corpus.md](../../docs/decoder/corpus.md).
