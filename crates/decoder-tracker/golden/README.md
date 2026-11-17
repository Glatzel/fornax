# Golden test and performance sets

These manifests are manually curated drafts. Maintainers may replace files as
long as every decoder path remains covered.

- `test-manifest.csv` is the fast correctness gate. It includes one smallest
  representative per tier-0/1 lossless decoder path plus regression and layout
  edge cases.
- `perf-manifest.csv` is the performance gate. It selects 1-3 stable files per
  path, with `overhead_only=1` marking small fixed-cost probes.
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
4. Cross-check the raw mosaic against Rawler and FlashRaw. Do not record a
   Fornax-only hash unless no second decoder supports the input.
5. Run `cargo run -p decoder-tracker -- golden-check`.

All payloads remain Git LFS pointers in committed files. RAW data is
transient and must never be committed, cached as an artifact, or uploaded.
CI shares one pinned raw.pixls.us LFS-object cache between the golden test job
and all decoder-benchmark matrix jobs.
