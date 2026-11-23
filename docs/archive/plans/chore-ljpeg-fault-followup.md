> Status: archived completed plan.
> Merged: #730 `chore/ljpeg-fault-followup` on 2026-11-15.
> Current guidance: [current documentation](../reference/decoders/perf/dng-remaining-gap.md).

# LJPEG Fault Follow-up Implementation Plan

> **For agentic workers:** This plan is executed inline in the current session.
> Steps use checkbox syntax so measurement and implementation state stay
> explicit.

**Goal:** Explain and, only where supported by real-file measurements, reduce
the remaining DNG LJPEG and floating-point Deflate cost after PR #728. Preserve
the owned-`Vec` API and exact output.

**Architecture:** First harden and test the shared large-integer allocator.
Then use the existing exact-input DNG probe with DWARF and `perf` fault call
graphs to attribute LJPEG faults at one and all threads. Prototype worker-local
scratch only if temporaries dominate. Separately repeat the float huge-page
experiment and audit output allocations. Finally rerun Fornax and FlashRaw in
the same session, update the report, validate, and hand off one pull request.

**Tech Stack:** Rust 2024, Rayon, libc on Linux, perf with DWARF call graphs
and annotation, the existing DNG comparison adapters, and synthetic LJPEG
fixtures where real inputs are not needed.

**Spec:** The latest user request in this thread and
`docs/reference/decoders/perf/dng-remaining-gap.md`.

## Global Constraints

- Page-cache-hot inputs, release mode, and the report host settings.
- Single-thread runs set `RAYON_NUM_THREADS=1`; all-thread runs use all 15
  visible CPUs and set FlashRaw's thread count explicitly.
- Every accepted real-file timing must pass exact dimensions, storage type,
  sample bits, and pixel-bit equality across Fornax, Rawler, and FlashRaw.
- One hypothesis per change. Benchmark before and after with fresh processes,
  warmups, and enough rounds to expose noise; revert anything within noise.
- Prefer safe Rust. Any unsafe allocation or FFI must have a concrete
  measured benefit and keep the owned-`Vec` API.
- Do not change LJPEG computations without predictor-1 annotation evidence.
- Do not claim portability. Linux huge-page behavior remains unmeasured on
  Windows and macOS.
- Do not run full workspace validation unless requested; run focused tests,
  formatting, clippy, and task-specific probes.

## Files Expected to Change

- `crates/fornax-codec/src/samples.rs`
- `crates/fornax-codec/src/deflate.rs` only if a measured float allocation
  change wins
- `crates/fornax-codec/src/ljpeg.rs` only if fault attribution justifies a
  scratch change
- `crates/fornax-codec/tests/encode_contract.rs` or a focused codec test file
- `crates/fornax-raw/benches/dng_gap_probe.rs`
- `docs/reference/decoders/perf/dng-remaining-gap.md`
- `docs/reference/decoders/perf/dng-gap-profiles/*`
- This plan

## Task 1: Allocator Soundness

- [x] Verify `Layout::array::<T>(len)`, `len`/`capacity`, overflow, and
  zero-length behavior against the current helper.
- [x] Make allocation failure call `handle_alloc_error`, and cfg out
  `madvise` under Miri.
- [x] Add a focused test under a counting global allocator that observes the
  large allocation and its matching deallocation.
- [x] Run codec tests before timing any subsequent change.

## Task 2: Remaining LJPEG Fault Attribution

- [x] Build the exact-input probe with DWARF in release mode.
- [x] Record `perf record -e minor-faults` call graphs for 5D3 and Pixel 8 at
  one thread and all 15 threads after page-cache warmup.
- [x] Classify fault stacks into full output first-touch, tile row scratch,
  Huffman tables, parse buffers, source mapping, thread stacks, allocator
  metadata, and unattributed paths.
- [x] If per-tile or per-row temporaries dominate, implement one per-worker
  scratch-reuse prototype and compare with `perf stat` and wall time.
- [x] Keep the prototype only if it improves both one-thread and all-thread
  real-file timings outside observed noise and equality remains exact.

## Task 3: Floating-point Huge-page Retry and Output Audit

- [x] Add a probe-only integration point that exercises the production
  `zeroed_samples::<f32>` allocation so `AnonHugePages`, minor faults, and wall
  time can be reported without changing production behavior.
- [x] Compare the current F32 allocation with the Linux natural-align
  `alloc_zeroed` plus `MADV_HUGEPAGE` policy on the FP32 Deflate file.
- [x] Audit `fornax-codec` output allocations and identify any decode path that
  can allocate an output of at least 8 MiB outside the shared helper.
- [x] Route additional large outputs through the helper only when they are
  decoded image outputs, not scratch, and keep the change only if the measured
  float-file result wins outside noise.
- [x] Add a regression test that exercises the shared allocator boundary for a
  large output without imposing a fragile hidden-branch assertion on unrelated
  APIs.
- [x] Re-measure FP16 and FP32 Deflate at one and all threads if production
  behavior changes.

## Task 4: Same-session FlashRaw Ratios and Predictor Evidence

- [x] Rerun Fornax and FlashRaw for every report-table file in one page-cache
  hot session, using identical thread settings.
- [x] Run the three-way equality gate for every DNG case before accepting the
  same-session ratios.
- [x] Locate and link the existing predictor-1 `perf annotate` evidence; if it
  is absent, capture it with the current DWARF binary.
- [x] Do not start additional LJPEG micro-optimizations without a new
  annotation-backed hypothesis. The previous two candidates remain reverted.

## Task 5: Documentation and Validation

- [x] Update the report with host and THP settings, fault-attribution tables,
  float retry evidence, same-session ratios, and kept/reverted changes.
- [x] Record exact commands and raw measurement artifacts under
  `docs/reference/decoders/perf/dng-gap-profiles/`.
- [x] Run `cargo test -p fornax-codec --all-features`, the synthetic LJPEG
  self-check, strict real-file equality, nightly formatting, focused clippy,
  and `git diff --check`.
- [x] Review the diff, commit, push, and open one semantic pull request.

## Validation

- `cargo test -p fornax-codec --all-features`
- `cargo bench -p fornax-codec --bench ljpeg_gap_probe -- --self-check`
- Strict DNG equality through `dng_gap_probe equality PATH`
- Release `perf stat` and DWARF `perf record` for the measured cases
- Nightly formatting and focused clippy
- `git diff --check`

Full workspace validation is not planned for this performance-focused change.

## Decisions and Notes

- Baseline codec tests passed before edits: 99 tests across 6 suites.
- The current helper uses `alloc_zeroed(Layout::array::<T>(len))` and transfers
  exactly `len` elements to `Vec::from_raw_parts`, so capacity equals length.
- The current null-allocation branch falls back to `vec![T::zeroed(); len]`.
  This is safe but does not directly satisfy the requested
  `handle_alloc_error` behavior and will be corrected.
- The current `madvise` call is compiled on every Linux target, including
  Miri. It will be excluded under Miri.
- LJPEG fault attribution found that full-image output and row scratch are no
  longer dominant. Per-tile Huffman lookup allocation and initialization
  account for about 87% of 5D3 and 80% of Pixel 8 one-thread fault samples.
- Reusing the per-worker Huffman buffers reduced 5D3 faults from 30,683 to
  3,033, but wall time was neutral on 5D3 and regressed 1.9% on Pixel 8 at 15
  threads. The prototype is reverted, matching the prior table-reuse result.
- The corrected float experiment reported 86 MiB of `AnonHugePages`, reduced
  one-thread faults by about 90-92%, and improved FP16/FP32 Deflate at both
  thread counts. Large float and remaining full-image JPEG/JPEG XL outputs now
  use the shared allocator.
- Same-session ratios were regenerated for every available headline file.
  Pentax K-5 remains unavailable in this checkout.
