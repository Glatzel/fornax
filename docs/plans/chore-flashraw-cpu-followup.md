# FlashRaw CPU Follow-up Plan

> **For agentic workers:** Do not implement until the source-comparison step is
> reviewed and a post-#737 baseline is selected.

**Goal:** Use FlashRaw 0.2.0's scalar CPU design as evidence for at most three
bounded Fornax experiments, without copying source or changing the public API.

**Architecture:** Start from the exact PR #738 allocator branch, compare it with
the pinned FlashRaw CPU paths, then rebase or synchronize with #737 before
measuring hypotheses that touch LJPEG compute. Every candidate is one isolated
change with exact equality, same-session FlashRaw, cycles/sample, wall time, and
revert-on-noise.

**Spec:** The user request in this thread and
[FlashRaw CPU source comparison](../reference/decoders/flashraw-cpu-comparison.md).

## Global Constraints

- FlashRaw 0.2.0 is MIT OR Apache-2.0; borrow ideas only and attribute the
  version, source path/line, and licence in any implementation comment.
- Ignore FlashRaw SIMD and GPU paths; compare scalar `Backend::Cpu` only.
- Do not copy FlashRaw code blocks.
- No public API or caller-owned-buffer API change without a separate proposal
  and explicit approval.
- Keep all platform-specific code and unsafe operations inside
  `crates/fornax-codec/src/large_alloc.rs`.
- Change one production hypothesis at a time.
- Every candidate must pass exact three-way dimensions, storage, bits, and
  pixel equality before timing.
- Use three fresh processes x five interleaved rounds, same-session FlashRaw,
  cycles/sample, wall median, and min-max spread.
- Revert anything within noise or with any repeatable regression.
- Do not compare measurements across host sessions.

## Step 0: FlashRaw CPU Source Comparison

- [x] Read the pinned FlashRaw 0.2.0 scalar LJPEG and DNG source.
- [x] Record Huffman layout/size, sign extraction, bit-pump state and refill,
  marker handling, row/predictor structure, output ownership, per-tile reuse,
  and allocator policy.
- [x] Record that FlashRaw and PR #738 already use the same 8 MiB natural-
  alignment huge-page policy.
- [x] Rank testable hypotheses and identify the public sink/output API as a
  separate proposal.
- [ ] Review
  [the source comparison](../reference/decoders/flashraw-cpu-comparison.md).

## Baseline Reconciliation

PR #738 is now merged with `origin/main` at `b52df26`, including #737's compact
Huffman path.

- [x] Synchronize PR #738 with `origin/main`.
- [x] Repeat the source comparison against the synchronized LJPEG code.
- [x] Mark compact 10-bit Huffman lookup as completed by #737 rather than
  re-running it.

## Task 1: Component-Pair Refill

- [ ] Add a failing test that counts refill boundaries for a two-component row.
- [ ] Implement one refill per component pair while preserving every stuffing,
  marker, truncation, and 16-bit-code test.
- [ ] Measure 5D3, Pixel 8, and one-component synthetic scans.
- [ ] Keep only if cycles/sample and wall time improve outside noise.

Expected: 1-3% cycles/sample and 1-2% wall time on two-component scans; no
change on one-component scans.

## Task 2: Lookup-Width/Full-Difference Experiment

- [ ] Select the current fixed table and one adaptive 10/12-bit full-difference
  variant; do not revive the old 16-bit fallback.
- [ ] Measure table-build cost, L1 misses, cycles/sample, and wall time.
- [ ] Keep only if the real-file result is outside noise with no noisy-scan
  regression.

Expected: 2-5% LJPEG wall improvement if full-hit mass justifies the wider
table; otherwise neutral or worse.

## Task 3: Single-Scan Parallelism

- [ ] First run the existing probe on the target single-strip scan and confirm
  that tile-level parallelism is insufficient.
- [ ] Prototype internal predictor-1 entropy speculation only if the target
  has enough work to amortize it.
- [ ] Do not expose a new public trait or buffer API.
- [ ] Keep only if 15T wall time improves materially and correctness falls back
  exactly for every unsupported/corrupt stream.

Expected: potentially 1.5-2x at 15T for a single large scan, no 1T gain, and
little or no gain on tiled DNGs.

## Proposed but Not Approved

A public caller-owned sink or decode-into trait would mirror FlashRaw's internal
output design more directly. It is not required by the current owned-return API,
and the common PR #738 LJPEG path already writes rows directly into its final
owned output. Record it as an API proposal only and stop if a candidate truly
requires it.

## Files Expected to Change

- `crates/fornax-codec/src/ljpeg.rs` for accepted LJPEG hypotheses
- `crates/fornax-codec/src/large_alloc.rs` only if allocator invariants change
- `crates/fornax-codec/benches/ljpeg_gap_probe.rs` for synthetic refill/table
  evidence
- `crates/fornax-raw-decoder/benches/dng_gap_probe/linux.rs` only for
  measurement output
- `docs/reference/decoders/flashraw-cpu-comparison.md`
- `docs/reference/decoders/dng-gap-profiles/README.md`
- `docs/plans/chore-flashraw-cpu-followup.md`

## Validation

```bash
cargo test -p fornax-codec --all-features
cargo bench -p fornax-codec --bench ljpeg_gap_probe -- --self-check
CARGO_PROFILE_RELEASE_DEBUG=line-tables-only \
  cargo build -p fornax-raw-decoder --release --bench dng_gap_probe
```

Run the exact three-way equality gate for every real file before accepting a
timing. Run repository validation only after a candidate is retained.

## Decisions and Notes

- FlashRaw allocator policy and PR #738 agree: natural-alignment
  `alloc_zeroed`, 8 MiB threshold, 2 MiB-aligned interior, Linux
  `MADV_HUGEPAGE`, plain fallback elsewhere.
- FlashRaw's 12-bit full-difference LUT, batched refill, and output sink are
  design observations only; no source was copied.
- The direct sink API is deferred as a public API proposal.
- No production code was changed in this source-comparison phase.
