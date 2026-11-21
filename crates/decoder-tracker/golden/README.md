# Golden test and performance sets

These manifests are manually curated drafts. Maintainers may replace files as
long as every decoder path remains covered.

- `test-manifest.csv` is the correctness gate. It includes one smallest
  representative per tier-0/1 lossless decoder path, representative lossless
  files for every supported native format, and explicit regression and layout
  edge cases such as Sony ARW6.
- `perf-manifest.csv` is the performance gate. It selects 1-3 stable files per
  path, with `overhead_only=1` marking small fixed-cost probes. Its `group`
  column is `core`, `full`, `overhead`, or `hdr`; benches default to `core`.
- `decode-hashes.csv` is shared by the union of both manifests. Existing rows
  are preserved when files are added.
- `selected-manifest.csv` is a legacy three-column alias of the test set for
  one release.

The `decoder_path` column uses the `golden-coverage` key. ARW and NEF aggregate
supported bit depths; DNG keeps its unpacking kernel, compression, sample type,
storage, predictor, and compression ID where applicable.

## Add a file

1. Confirm the row is fully passing, lossless/uncompressed, and present in
   `data/baseline.csv` and `data/manifest.csv`.
2. Add its pointer metadata, decoder path, and short reason to the relevant
   manifest. Use `overhead_only=1` only for a small perf probe.
3. Generate its shared hash with
   `cargo run --release -p decoder-tracker -- hash --manifest MANIFEST`.
4. Cross-check the raw mosaic against Rawler and FlashRaw when their raw-domain
   semantics match. For vendor-normalized paths such as IIQ sensor correction
   or RW2 black offset, record the Fornax hash and state the intentional
   difference in the manifest reason or decoder report.
5. Run `cargo run -p decoder-tracker -- golden-check`.
6. Regenerate and verify the shared hashes once. Routine repository validation
   leaves the full materialized-hash pass to CI.

Select a benchmark group with `FORNAX_BENCH_GROUP=core` (the default),
`full`, `overhead`, `hdr`, or `all`. A comma-separated list is accepted when
more than one group is needed. `golden-check` prints the core byte count and a
rough probe-time estimate; the estimate assumes 21 decodes per file at
64 MiB/s.

All payloads remain Git LFS pointers in committed files. RAW data is
transient and must never be committed, cached as an artifact, or uploaded.
CI shares one pinned raw.pixls.us LFS-object cache between the golden test job
and all decoder-benchmark matrix jobs.
