> Status: archived completed plan.
> Merged: #727 `chore/dng-first-touch-probe` on 2026-11-15.
> Current guidance: [current documentation](../../reference/decoders/perf/dng-remaining-gap.md).

# DNG First-Touch and LJPEG Follow-up

## Goal

Determine whether first-touch allocation policy explains the remaining DNG gap
to FlashRaw on 5D3 uncompressed U16 and floating-point Deflate, using a probe
that is verified to change page faults. Retain a production allocation change
only if an owned-`Vec` policy wins outside measurement noise. Separately test a
single fused predictor-1 LJPEG kernel and multi-thread scaling, with no decode
change retained unless wall time and instructions per sample both improve.

## Scope

In scope:

- Record the host, kernel, glibc, and transparent-hugepage settings.
- Capture FlashRaw counters for 5D3 uncompressed, 5D3 LJPEG, and 5D3 FP32
  Deflate, and inspect FlashRaw's source allocation.
- Replace the invalid page-alignment probe with an anonymous 2 MiB-aligned
  mapping whose pages are advised before first write.
- Verify transparent-huge-page backing through `AnonHugePages` and verify that
  minor faults change relative to the fresh baseline.
- Compare fresh allocation, prefaulted owned `Vec`, prefaulted plus
  `MADV_HUGEPAGE`, and unchanged code under the glibc hugetlb tunable.
- Reconcile the 5D3 uncompressed end-to-end time from source faults, TIFF
  assembly, allocation/first touch, sample copy, and residual overhead.
- If an allocation policy wins outside noise, add the smallest Linux-only
  owned-`Vec` implementation, then run the three-way equality gate and
  re-measure selected uncompressed, Deflate, and LJPEG files.
- Capture a post-change LJPEG DWARF profile and prototype one fused predictor-1
  kernel in a benchmark.
- Measure Fornax and FlashRaw multi-thread scaling on tiled LJPEG and Deflate
  files.
- Update the DNG remaining-gap report with host-specific before/after numbers,
  profile evidence, and retained/reverted decisions.

Out of scope:

- General-purpose allocator replacement or caller-owned decode buffers.
- Decode API changes.
- Correctness or architecture changes unrelated to measured hot paths.
- Full workspace validation; the user requested validation be skipped.
- Portable performance claims.

## Task Checklist

- [x] Record host and THP configuration under the report artifacts.
- [x] Capture FlashRaw perf counters and source allocation behavior.
- [x] Repair and independently verify the aligned hugepage probe.
- [x] Measure fresh, prefaulted, prefaulted-hugepage, and glibc-tunable paths.
- [x] Reconcile the 5D3 uncompressed stage table with the 52 ms baseline.
- [x] Decide and implement the measured integer-output allocation policy.
- [x] Re-run equality and end-to-end before/after measurements for selected files.
- [x] Profile post-C LJPEG and reject the fused predictor-1 prototype.
- [x] Measure all-thread scaling against FlashRaw.
- [x] Update the report and review the final diff.

## Files Expected to Change

- `crates/fornax-raw-decoder/benches/dng_gap_probe.rs`
- `crates/fornax-codec/benches/ljpeg_gap_probe.rs`
- `crates/fornax-codec/src/samples.rs` only if the allocation hypothesis wins
- `docs/reference/decoders/perf/dng-remaining-gap.md`
- `docs/reference/decoders/perf/dng-gap-profiles/` measurement artifacts
- `docs/plans/chore-dng-first-touch-probe.md`

## Validation

The corrected probe must prove output equality before timing and empirically
demonstrate a fault-count or `AnonHugePages` change before its result is
accepted. Timings use release mode, page-cache-hot inputs, one thread for the
single-thread comparison, and repeated interleaved processes. Any production
decode change requires the exact dimensions, storage type, sample bits, and
pixel-value three-way equality gate. Full workspace validation is intentionally
not run for this task.

## Decisions and Notes

- The prior hugepage probe is treated as invalid until corrected: its
  over-aligned `alloc_zeroed` region was initialized before `madvise`, its
  reported fault count did not change, and only its interior allocation was not
  verified as huge-page backed.
- FlashRaw uses natural-alignment `std::alloc::alloc_zeroed`, advises a
  rounded 2 MiB-aligned interior, and does not touch the output before decode.
- One hypothesis is changed and benchmarked at a time. Improvements within
  observed noise are reverted.
- The corrected probe produced 43-45 MiB of `AnonHugePages` and reduced 5D3
  output faults from about 11,600 to about 870 per isolated round.
- The natural `alloc_zeroed` ownership transfer was retained over the safe
  double-zero-fill `Vec::with_capacity` policy: it measured about 8.5-9.1 ms
  versus 12-15 ms and materially fewer instructions.
- The policy is restricted to integer outputs of at least 8 MiB. A broader
  floating-point policy was neutral on FP32 and regressed FP16 by about 1.5-2%.
- The fused LJPEG prototype helped one component in isolation but regressed
  two-component wall time and instructions/sample, so it was removed.
