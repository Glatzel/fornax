> Status: archived completed plan.
> Merged: #728 `chore/dng-followup-probes` on 2026-11-15.
> Current guidance: [current documentation](../../reference/decoders/perf/dng-remaining-gap.md).

# DNG Follow-up Performance Probes Implementation Plan

> **For agentic workers:** This plan is executed inline in the current session.
> Steps use checkbox syntax so measurement and implementation state stay
> explicit.

**Goal:** Close or explain the remaining DNG gaps whose causes were not settled
by the first-touch work, retaining only changes that beat the real-file baseline
outside noise while preserving exact output.

**Architecture:** Measure each allocation and compute hypothesis against the
current C-only LJPEG path before changing it. Reuse the existing exact-input
probe for timing and equality, and add narrowly scoped benchmark-only modes for
fault attribution and scratch reuse. Production changes are isolated to the
allocator boundary, optional one-time LJPEG unstuffing, and at most two
annotate-driven kernel changes.

**Tech Stack:** Rust 2024, Rayon, bytemuck, libc on Linux, perf with DWARF call
graphs and annotation, existing Synthetic LJPEG and DNG comparison adapters.

**Spec:** User follow-up request in this thread plus
`docs/reference/decoders/perf/dng-remaining-gap.md`.

## Global Constraints

- Page-cache-hot inputs, release mode, and the report host settings.
- Single-thread comparisons use `RAYON_NUM_THREADS=1`; 15-thread comparisons
  use all visible CPUs.
- Keep exact dimensions, storage type, sample bits, and pixel bits equal across
  Fornax, Rawler, and FlashRaw before accepting a timing.
- One hypothesis per production change; benchmark before and after.
- Revert any wall-time or instruction improvement within observed noise.
- Prefer safe Rust. Any additional unsafe must have a measured justification.
- Linux huge-page behavior is not assumed on Windows or macOS.
- Do not run full workspace validation unless requested; run focused tests and
  compile checks.

## Review Focus

- LJPEG direct-row decoding must not accidentally bypass a shared large-output
  allocator while fallback tile decoding still works.
- Unstuff scratch reuse must preserve marker termination, `0xff00` handling,
  one- and two-component scans, and per-tile output bits.
- Rayon worker-local scratch must not leak state between scans or change tiled
  output order.
- Deflate temporary reuse must preserve malformed/truncated payload behavior.
- Annotation-driven changes must not specialize only synthetic smooth scans or
  regress noisy real files.

---

## Task 1: LJPEG Large-Output Allocation

**Files:**

- Modify: `crates/fornax-codec/src/samples.rs`
- Modify: `crates/fornax-codec/src/ljpeg.rs`
- Modify only for probe support if needed:
  `crates/fornax-raw-decoder/benches/dng_gap_probe.rs`

**Interfaces:**

- Consumes: `zeroed_samples<T: Pod>(len: usize) -> Vec<T>` and
  `DecodeCompression::LosslessJpeg`.
- Produces: LJPEG full-image and interleaved output allocations that use the
  same integer-output policy as other codecs.

- [x] **Step 1: Capture current LJPEG timing and fault counters**

Run three fresh processes per decoder for 5D3 and Pixel 8 at one thread, then
the same two Fornax cases at 15 threads. Capture `minor-faults`, `page-faults`,
and wall time. Run the strict equality mode for both files.

- [x] **Step 2: Confirm bypass at both LJPEG allocation sites**

Record that `ljpeg::decode` allocates its full image with
`vec![0_u16; sample_count]`, and `decode_interleaved_impl` uses the same pattern
for standalone tile decoding. Determine whether only the first site is large
enough on the real files.

- [x] **Step 3: Expose and apply the allocation helper**

Make `samples::zeroed_samples` crate-visible and replace the LJPEG U16 output
allocations with it. Do not change sample values or ownership types.

- [x] **Step 4: Run focused tests and equality**

Run LJPEG tests and strict equality on 5D3 and Pixel 8.

- [x] **Step 5: Re-measure 1T and 15T**

Use identical process counts and counters. Keep the change only if LJPEG wall
time or faults improve meaningfully without a noisy-case regression.

## Task 2: Reusable Unstuff Scratch for Hypothesis A

**Files:**

- Modify: `crates/fornax-codec/src/ljpeg.rs`
- Modify: `crates/fornax-codec/benches/ljpeg_gap_probe.rs`
- Modify: `crates/fornax-raw-decoder/benches/dng_gap_probe.rs` only for
  counter-only invocation support

**Interfaces:**

- Consumes: `FastBitReader` and a tile entropy slice.
- Produces: a benchmark-only `unstuffed` mode and, if retained, a production
  path that unstuffs exactly once into worker-local storage before the plain
  bit pump runs.

- [x] **Step 1: Add a failing stuffed/unstuffed equality test**

Encode scans containing `0xff00`, marker termination, and both component counts;
decode through the current path and the prototype unstuffed path; assert exact
sample equality and identical error behavior.

- [x] **Step 2: Implement benchmark-only per-worker scratch reuse**

Use a Rayon worker-local `RefCell<Vec<u8>>` or equivalent reusable scratch.
Clear and resize the scratch per tile, unstuff once, then run a marker-check-free
bit reader over the plain bytes.

- [x] **Step 3: Benchmark A+C against C**

Measure one and two components, smooth and noisy, at one and 256 tiles. Record
wall time and minor faults. Run 5D3 and Pixel 8 at one thread.

- [x] **Step 4: Keep or revert**

Retain only if smooth/noisy and both real files avoid a regression outside
noise. If noisy scans still regress, remove the prototype and record the
failure.

## Task 3: Deflate Minor-Fault Attribution

**Files:**

- Create:
  `docs/reference/decoders/perf/dng-gap-profiles/5d3-fp32-deflate-minor-faults.txt`
- No production file changes before attribution.

**Interfaces:**

- Consumes: FP32 Deflate decode and perf `minor-faults` call graphs.
- Produces: fault shares for source mapping, full output, inflate scratch,
  per-strip/tile temporaries, allocator metadata, and other.

- [x] **Step 1: Capture a minor-fault DWARF profile**

Run `perf record -e minor-faults:u --call-graph dwarf,16384` on repeated FP32
decode after page-cache warmup.

- [x] **Step 2: Attribute fault stacks**

Classify each fault by allocation site and report counts and shares. Include
FP16 as a confirmation run if the FP32 split needs disambiguation.

- [x] **Step 3: Decide whether scratch reuse is justified**

Only prototype per-worker scratch reuse if inflate or per-strip temporaries
dominate. Measure FP16/FP32 at one and 15 threads and revert if within noise.

## Task 4: Annotate the LJPEG Predictor-1 Closure

**Files:**

- Create:
  `docs/reference/decoders/perf/dng-gap-profiles/ljpeg-predictor-annotate.txt`
- Modify only if supported by annotation:
  `crates/fornax-codec/src/ljpeg.rs`
- Modify only if supported by annotation:
  `crates/fornax-codec/benches/ljpeg_gap_probe.rs`

**Interfaces:**

- Consumes: the DWARF-enabled C-only binary and 5D3 LJPEG profile.
- Produces: instruction shares for refill, marker checks, lookup, sign
  extension, stores, and loop/index work, plus at most two target proposals.

- [x] **Step 1: Record annotated hot ranges**

Use `perf annotate --stdio` for the inlined closure and map instruction samples
to source operations. Compare resulting instructions/sample with FlashRaw's
951,622,276 instructions per 23,384,000-sample decode.

- [x] **Step 2: Propose at most two changes**

Each proposal must cite the annotated operation and predicted savings. Do not
start implementation if the annotation cannot distinguish competing causes.

- [x] **Step 3: Benchmark each change alone**

Run synthetic one/two-component smooth/noisy scans and real 5D3/Pixel. Record
instructions/sample and wall time. Revert any proposal without improvement in
both.

## Task 5: Documentation and Final Evidence

**Files:**

- Modify: `docs/reference/decoders/perf/dng-remaining-gap.md`
- Create or update measurement files under
  `docs/reference/decoders/perf/dng-gap-profiles/`

- [x] **Step 1: Correct stale text**

Replace the stale Deflate `1.23x-1.31x` ratio with the current measured values
and replace the outdated “Next Probes” section with the completed open-question
results.

- [x] **Step 2: Record all retained and reverted decisions**

Include before/after 1T and 15T tables, minor-fault counters, equality status,
host/THP settings, and the Linux-only/Windows-unmeasured limitation.

- [x] **Step 3: Run focused validation**

Run codec tests, LJPEG generator self-check, strict real-file equality, formatting,
and focused clippy. Do not run full workspace validation.
- [x] **Step 4: Review the diff, commit, push, and open the pull request**

## Files Expected to Change

- `crates/fornax-codec/src/samples.rs`
- `crates/fornax-codec/src/ljpeg.rs`
- `docs/reference/decoders/perf/dng-remaining-gap.md`
- `docs/reference/decoders/perf/dng-gap-profiles/*`
- `docs/plans/chore-dng-followup-probes.md`

## Validation

- `cargo test -p fornax-codec --all-features`
- Synthetic LJPEG generator self-check
- Strict three-way equality for 5D3 and Pixel 8 LJPEG
- Nightly formatting and focused clippy
- Full workspace validation is not run.

## Decisions and Notes

- The pulled baseline contained a broken `Box::from(slice::from_raw_parts(...))`
  ownership transfer. It was restored to `Vec::from_raw_parts`, which both
  compiles and adopts the measured huge-page allocation.
- LJPEG allocation reuse is retained.
- Reusable unstuff scratch is reverted because noisy scans still regress.
- Deflate scratch reuse is not implemented because output faults dominate.
- Both annotation-driven predictor changes are reverted after real-file
  regressions.
