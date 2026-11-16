> Status: archived completed plan.
> Merged: #735 `chore/arw-gap-profiles` on 2026-11-15.
> Current guidance: [current documentation](../../reference/decoders/perf/arw-remaining-gap.md).

# Sony ARW Gap Profiles

## Goal

Measure portable-scalar Sony ARW decode performance for the corpus's
uncompressed and real lossless-JPEG cases, with exact three-way output gates,
one- and fifteen-thread timings, counters, stage attribution, and a
DNG-comparison report.

## Scope

In scope:

- Inventory all ARW corpus rows and verify candidate lossless streams by parsing
  their JPEG marker stream.
- Materialize and jointly validate only the uncompressed and real lossless-JPEG
  cases needed for benchmarking.
- Compare Fornax `Decoder::decode(path).raw_mosaic()`, Rawler 0.8.0
  `decode_file`, and FlashRaw 0.2.0 CPU decode.
- Record release-mode, page-cache-hot, three-process/five-round timing and
  process counters at one and fifteen threads.
- Attribute uncompressed and lossless-JPEG stages, identify codec paths, and
  propose portable optimizations without implementing them.

Out of scope:

- Lossy Sony ARW2/ARW6, delta compression, medium/small reduced-resolution
  formats, unsupported layouts, and any decoder behavior change.
- Platform-specific allocation, libc advice, SIMD, target features, or new
  `fornax-codec` dependencies.
- Portable performance guarantees.

## Task Checklist

- [x] Initialize the current pointer-only corpus snapshot and verify its manifest.
- [x] Select the ARW inventory from the manifest and baseline.
- [x] Materialize the uncompressed and lossless candidate ARWs with the tracker.
- [x] Extract representative metadata with `exiftool` and verify LJPEG SOI/SOF3.
- [x] Preflight Fornax, Rawler, and FlashRaw support and three-way equality.
- [x] Measure one and fifteen threads for the three trusted representatives.
- [x] Collect Fornax and FlashRaw instructions, IPC, branch misses, and faults.
- [x] Attribute uncompressed and LJPEG stages and verify codec paths.
- [x] Write the report and linked text artifacts.
- [x] Run documentation validation and inspect the final diff.

## Files Expected to Change

- `docs/reference/decoders/perf/arw-remaining-gap.md`
- `docs/reference/decoders/perf/arw-gap-profiles/*.txt`
- Possibly `docs/README.md` if the documentation index requires the new report.
- Transient benchmark harness code may be used under `crates/fornax-raw-decoder/`
  during measurement, but it is removed before the final diff unless the report
  needs it for reproducibility.

## Validation

```bash
scripts/agent-validation.sh --docs-only
git diff --check
```

Before accepting a timing ratio, require exact dimensions, storage type, sample
bits, and pixel values across Fornax, Rawler, and FlashRaw. Record unsupported
inputs as `unsupported`, not as failed or timed results.

## Decisions and Notes

- The pinned corpus reports 40 uncompressed ARWs and 19 streams classified as
  Sony lossless JPEG. Inventory will use a `manifest.csv` join to
  `baseline.csv`; payloads for lossy/delta rows will not be materialized.
- The raw.pixls.us pinned submodule commit is no longer fetchable because its
  daily LFS history was replaced. Measurements use the already-advanced
  2026-11-08 snapshot whose paths and sizes still match the committed manifest.
- After the user requested fewer downloads, all 206 ARW rows remain in the
  inventory. Eight files were gated: three trusted timing representatives plus
  five 12-bit uncompressed diagnostics. The three trusted files pass; all five
  12-bit diagnostics fail exact pixel equality.
- The headline input is page-cache-hot. Each process runs one warmup and five
  measured rounds, and three fresh processes are interleaved per decoder.
- FlashRaw uses `Backend::Cpu`, strict corruption handling, no correction
  opcodes, no crop, and the requested worker count.
- Host-specific measurements are evidence, not portable performance
  guarantees.
