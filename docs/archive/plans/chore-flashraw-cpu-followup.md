> Status: archived completed plan.
> Merged: #738 `chore/large-page-allocation` on 2026-11-16.
> Current guidance: [current documentation](../reference/decoders/perf/dng-remaining-gap.md).

# FlashRaw CPU Follow-up Plan

> **For agentic workers:** The source comparison, baseline, and bounded
> candidates are complete. Do not implement single-scan parallelism until its
> written boundary design is separately approved.

**Goal:** Establish the post-#737/#738 LJPEG baseline, then evaluate at most
three bounded candidates in this order: fixed 12-bit packed-u32 Huffman table,
component-pair refill, and restart-free single-scan parallelism after a written
design.

**Architecture:** Start from the exact PR #738 allocator branch, compare it with
the pinned FlashRaw CPU paths, and use the branch after its merge with #737 as
the measurement baseline. Every candidate is one isolated change with exact
equality and same-session FlaskRaw. Cycles/sample and wall time decide keep or
revert; instructions/sample and L1D misses/sample are always reported as
supporting evidence.

**Spec:** The user request in this thread and
[FlashRaw CPU source comparison](../reference/decoders/perf/flashraw-cpu-comparison.md).

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
- Use three fresh processes x five interleaved rounds and same-session FlashRaw.
- Judge every candidate primarily on cycles/sample and wall-time median with
  min-max spread. Always report instructions/sample and L1D load
  misses/sample alongside.
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
- [x] Review
  [the source comparison](../reference/decoders/perf/flashraw-cpu-comparison.md).

## Task 1: Post-#737/#738 Baseline

PR #738 is now merged with `origin/main` at `eaf7ec6`, including #737's compact
Huffman path.

- [x] Synchronize PR #738 with `origin/main`.
- [x] Repeat the source comparison against the synchronized LJPEG code.
- [x] Mark compact 10-bit Huffman lookup as completed by #737 rather than
  re-running it.

- [x] Materialize and record
  `external/raw.pixls.us/Pentax/K-5/_MAR0543.DNG`; include its hash and whether
  Rawler/FlashRaw pass the same strict equality gate as Fornax.
- [x] Record a clean same-session baseline for 5D3, Pixel 8, iPhone SE, and
  Pentax K-5 LJPEG strips/tiles at 1T and 15T. Include 5D3 uncompressed and
  FP16/FP32 Deflate as allocation-policy guards.
- [x] For every row, record wall median/min-max, cycles/sample,
  instructions/sample, L1D load misses/sample, and same-session FlashRaw.
- [x] Save only the summary table under
  `docs/reference/decoders/perf/dng-gap-profiles/`; do not commit raw perf files.

The Pentax K-5 case is mandatory because it is the motivating single-strip,
restart-free LJPEG workload. If its corpus object, Fornax decoder, or reference
decoder cannot run, stop before any optimization and report the exact missing
precondition. The file is materialized and all three decoders pass equality.
The evidence is in
[`flashraw-cpu-followup.txt`](../reference/decoders/perf/dng-gap-profiles/flashraw-cpu-followup.txt).

## Task 2: Fixed 12-Bit Packed-u32 Full-Difference Table

This must be tested before any adaptive width selection.

- [x] Add failing table tests for short full-difference hits, non-full boundary
  codes, long codes beyond 12 bits, 16-bit differences, invalid codes, marker
  stuffing, and truncation.
- [x] Add the fixed 12-bit lookup candidate: 4,096 packed-`u32` entries, exactly
  16 KiB per table. Entries carry consumed bits, full/valid state, symbol, and
  `i16` difference where the code plus magnitude fit. Use the compact canonical
  counts/symbols slow path for everything else. Do not add or restore the
  65,536-entry 16-bit fallback.
- [x] Compare the fixed 12-bit table directly with the post-#737 10-bit table
  on every Task 1 case at 1T and 15T.
- [x] Record cycles/sample, instructions/sample, L1D misses/sample, wall
  median/min-max, table-build cost, and same-session FlashRaw.
- [x] Revert the 12-bit table because Pentax repeated a clear regression. Do
  not proceed to adaptive width selection in this plan.

Result: the table reduced LJPEG instructions and improved tiled files, but
Pentax K-5 regressed from about 92.5 ms to 107.5 ms with higher cycles and L1D
pressure. It was reverted. Table construction remained part of the one-tile
synthetic decode; no separate rejected-code microbenchmark was retained.

Expected: 2-5% LJPEG wall improvement if large-difference rejection dominates
the 10-bit table; neutral or slower if its 16 KiB footprint increases L1D
pressure.

## Task 3: Component-Pair Refill

Start only after the Task 2 table decision.

- [x] Add a failing test that counts refill boundaries for a two-component row.
- [x] Implement one refill per component pair while preserving every stuffing,
  marker, truncation, and 16-bit-code test.
- [x] Measure every Task 1 case at 1T and 15T plus one- and two-component
  synthetic scans.
- [x] Record the complete metric set and revert it because cycles/sample and
  wall time regressed outside noise.

Result: paired targets reduced the synthetic refill count and preserved
equality, but raised real LJPEG instructions/sample from about 79.6 to 98.2 and
wall time by 8.5-14.0% at 1T. It was reverted.

Expected: 1-3% fewer cycles/sample and 1-2% wall time on two-component scans; no
measurable change on one-component scans.

## Task 4: Pentax/Single-Scan Parallelism Design Gate

No parallelism code starts here until a written design is reviewed and
approved.

- [x] Confirm from the Task 1 baseline that Pentax or another targeted
  single-strip scan lacks enough tile-level parallelism.
- [x] Write the internal design before code. It must state the exact method for
  finding safe work boundaries when no restart markers exist, including how
  candidate bit positions are selected, how table/value phase and predictor
  state are synchronized, how disagreements or corrupt speculation are
  detected, how results fall back to the exact sequential decoder, and how
  deterministic row ordering and output equality are preserved.
- [x] Document why the method cannot silently skip or duplicate symbols and why
  truncated/malformed streams retain current error semantics.
- [ ] Stop for design approval before implementation. The design currently
  recommends no-go because exact boundary discovery duplicates the hot Huffman
  walk without establishing an obvious win.
- [ ] If approved, implement only the designed internal path, with no public
  trait or caller-owned buffer API.
- [ ] Judge by cycles/sample and wall time at 1T and 15T, reporting instructions
  and L1D misses. Keep only if the target improves materially and no supported
  case regresses outside noise.

Expected: potentially 1.5-2x at 15T on a single large scan, no 1T gain, and
little or no gain on already-tiled DNGs.

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
- `crates/fornax-raw/benches/dng_gap_probe/linux.rs` only for
  measurement output
- `docs/reference/decoders/perf/flashraw-cpu-comparison.md`
- `docs/reference/decoders/perf/dng-gap-profiles/README.md`
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
- The post-#737/#738 baseline is `eaf7ec6`; Pentax K-5 passes strict equality.
- The fixed 12-bit table and component-pair refill were measured one at a time
  and both reverted under the no-regression gate.
- No production performance change from this follow-up is retained.
- Restart-free single-scan parallelism is stopped at its written design gate.
