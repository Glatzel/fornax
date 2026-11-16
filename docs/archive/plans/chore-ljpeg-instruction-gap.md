> Status: archived completed plan.
> Merged: #736 `chore/ljpeg-instruction-gap` on 2026-11-15.
> Current guidance: [current documentation](../reference/decoders/perf/dng-remaining-gap.md).

# LJPEG Instruction Gap Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox syntax so profiling, implementation, measurement, and reversion
> state stay explicit.

**Goal:** Confirm and test the reviewed LJPEG instruction-cost hypotheses one
at a time, retain only changes that beat measured noise, and document the final
portable scalar reader against same-session FlashRaw.

**Architecture:** Start from the merged PR #734 plain-`Vec` state and attribute
the predictor-1 hot loop to source lines before editing it. Each later
hypothesis is implemented on its own commit, measured against the fixed
baseline and same-session Fornax/FlashRaw processes, then either retained or
reverted in a separate commit. Tests and exact three-way equality gate every
retained change.

**Tech Stack:** Rust 2024, `fornax-codec`, the harness-free
`ljpeg_gap_probe`, the `dng_gap_probe` Linux bench, Rayon, `perf record`,
`perf annotate`, and the existing FlashRaw 0.2.0 and Rawler 0.8.0 benchmark
adapters.

**Spec:** The current user task. Baseline is commit
`65cbb3815ecce6061bca768a58b43b0890a51aaa` from `origin/main`.

## Global Constraints

- Use portable scalar Rust only. Do not use `cfg(target_os)`, `libc`,
  `madvise`, SIMD, intrinsics, `portable_simd`, or `target_feature`.
- Do not add dependencies.
- Do not use `as` truncation where the workspace clippy policy rejects it. Use
  small const helpers like `low_u16` and `low_u32`.
- Keep the owned-return public API unchanged.
- Do not change decoded behavior. Dimensions, storage, bits, and every sample
  must pass the three-way equality gate.
- Test one production hypothesis at a time. Do not stack an unmeasured change
  onto another.
- Run the same fixed baseline for every comparison. Use three fresh processes,
  five interleaved rounds per process, median and min-max.
- Measure 5D3 and Pixel 8 at one and fifteen threads, iPhone SE at one thread,
  and the synthetic one- and two-component smooth and noisy scans.
- Run FlashRaw in the same session as each Fornax comparison.
- Keep a candidate only when it beats the measured noise spread. Record a
  rejected attempt in the artifact, then revert it in its own commit.
- Stop at approximately 1.3x FlashRaw instructions per sample, or after two
  consecutive attempted steps fail to improve beyond noise.
- Save profiling and measurement artifacts under
  `docs/reference/decoders/perf/dng-gap-profiles/`, with the final human-readable
  report under `docs/reference/decoders/` (including updates to
  `dng-remaining-gap.md`).
- Record host, kernel, THP mode, and THP defrag mode, and label all timing
  results as host-specific.

## Scope

In scope:

- Step 0 source-line annotation, register/spill inspection, and temporary
  counter probes.
- One word-refill candidate, with reference/fuzz/stuffing/marker/truncation and
  16-bit-code tests.
- Reader-state-in-registers restructuring if Step 0 shows spills.
- Separately measured loop-overhead candidates: boundary peeling and
  point-transform specialization; local predictors plus
  `chunks_exact_mut`; and truncation/mask cleanup.
- Separately measured table candidates: 12-bit fused, 11-bit fused, and a
  two-level 9-10-bit code table with direct cache amplitude extraction.
- Conditional re-evaluation of unstuff-once, const-generic one/two-component
  loops, and the fused direct-band kernel only if Steps 1-4 materially change
  the profile.
- Final report, retained/reverted table, asm observations, and artifact files.

Out of scope:

- New SIMD, platform allocation, dependencies, or public APIs.
- Additional experiments not named in the task.
- Deflate, uncompressed DNG, demosaic, or unrelated decoder work.
- Full corpus tracker runs or RAW payload uploads.

## Files Expected to Change

- `crates/fornax-codec/src/ljpeg.rs`
- `crates/fornax-codec/benches/ljpeg_gap_probe.rs` only if measurement support
  is required
- `crates/fornax-raw-decoder/benches/dng_gap_probe/linux.rs` only if
  counter-only or exact-input measurement support is required
- `docs/reference/decoders/perf/dng-gap-profiles/*`
- `docs/reference/decoders/perf/dng-remaining-gap.md`
- `docs/plans/chore-ljpeg-instruction-gap.md`

## Review Focus

- Word refill must preserve `0xff00` stuffing, marker termination, zero fill
  after end, truncated streams, and codes up to 16 bits.
- The fast lookup must not read ahead across a marker as if marker bytes were
  entropy data.
- Reader locals passed to cold helpers must remain behaviorally identical to
  `&mut FastBitReader`, including the first recorded error.
- Peeling row zero, column zero, and nonzero point transform must not alter
  predictor selection or row swapping.
- Local predictor vectors must preserve interleaving, row boundaries, packed
  rows, and one/two-component scans.
- Table miss-rate measurements must describe actual fast lookup results, not
  only table construction.
- A synthetic-only gain that regresses a real file outside noise is rejected.

---

## Exact-count execution order (authoritative)

The exact Callgrind data ranks the work as follows. Sampled-profile
percentages are not used for ranking.

1. `apply_difference` and `peek_no_fill` truncation helpers.
2. Loop/predictor/component hoists and local predictors, measured as separate
   sub-steps.
3. Fixed-size fast table (`[Entry; 8192]`, index `& 0x1fff`) to remove the
   three-instruction slice bounds check.
4. Word refill.
5. Fast-entry packing only if the post-refill assembly still shows separate
   field-load pressure.
6. Reader state in locals only if the preceding assembly shows reader-field
   spills.

The measured reasons are:

- `apply_difference` is 10.000 instructions/sample, including five
  instructions emitted for `u16::try_from(...).expect(...)`.
- Loop/control plus predictor traffic is about 28.6 instructions/sample;
  repeated Vec indexing adds stack loads, bounds checks, and pointer reloads.
- The fast lookup core is 4.971 instructions/sample and its bounds check is
  3.000. Skip/peek/cache shifts are separate at 8.072.
- Refill is 21.951 instructions/sample, but it is no longer the cheapest
  first target because `apply_difference` has a behavior-preserving exact
  replacement and loop/predictor work removes multiple stack and bounds
  checks.
- Slow Huffman misses are 0.202 instructions/sample and 1.3-1.4% of samples;
  no extra two-level miss-removal experiment is justified.

### Revised tasks

- [x] **Rev Task 0: Exact profile and reconciliation**

  Use full-file Callgrind, produce the per-instruction table and lookup
  annotation, record reader counters, and reconcile the 25,165,824 padded
  LJPEG samples with the 23,384,000-sample cropped mosaic and the native
  ~100.7 instruction/sample figure.

- [x] **Rev Task 1: Apply-difference truncation helpers**

  Add failing boundary tests, replace `u16::try_from(...).expect(...)` in
  `apply_difference` and the conversion in `peek_no_fill` with
  `to_le_bytes`-style const truncation helpers, and drop the redundant mask
  when a right shift already yields exactly `count` bits. Measure exact
  before/after instructions and all required real/synthetic cases.

- [ ] **Rev Task 2: Loop, predictor, and component work**

  Measure independently: (a) hoist row-zero, column-zero, and point-transform
  branches; (b) keep component predictors in locals and write rows with
  `chunks_exact_mut(components)`; (c) attempt const-generic one/two-component
  row loops after (a) and (b). Each candidate is a separate commit and a
  separate keep/revert decision.

  Progress: sub-step 2a (peeled row/edge loops) was measured and reverted in
  `c4481e9`. Exact instructions regressed by 1,099,021 (+0.044 per decoded
  sample) and every real-file instructions/sample result regressed.

  Progress: sub-step 2b (component-local predictors and chunked row writes) is
  kept in `2740d3b`. Exact instructions fall by 209,083,144 (-8.308 per
  decoded sample); native instructions/sample fall by about 9-10%.

- [x] **Rev Task 3: Fixed-size fast table**

  Replace the 13-bit `Box<[FastHuffmanEntry]>` access with a fixed-size table
  and `index & 0x1fff`. Confirm the three-instruction bounds check disappears.
  Keep the current struct fields initially so table shape is measured alone.
  Kept in `82ad348`: -114,445,744 exact instructions (-4.547 per decoded
  sample), with native instr/sample improving about 5.9% on every real case.

- [x] **Rev Task 4: Word refill**

  Implement the checked eight-byte big-endian refill with SWAR `0xff`
  detection and a cold byte-preserving fallback, then remove inner refill
  calls. Cover stuffing periods, marker end, truncation, 16-bit Huffman codes,
  and leftover-lookahead composition.
  Kept in `3ef1c75`: -386,739,410 exact instructions (-15.367 per decoded
  sample). Candidate is 1.51x FlashRaw on 5D3, 1.48x on Pixel 8, and 1.46x on
  iPhone SE.

- [x] **Rev Task 5: Conditional entry packing and reader locals**

  Pack `FastHuffmanEntry` only if new assembly still performs separate
  `full`/`consumed`/`difference` loads and register pressure justifies it.
  Move reader state into hot-loop locals only if new assembly shows spills.
  Packed entry implemented and kept in `f7f4a79`: -99,903,939 exact
  instructions. Native ratios are now 1.41x, 1.38x, and 1.42x FlashRaw.
  Reader-local restructuring was not pursued because the final asm keeps reader
  state in registers; remaining stack traffic is Vec/loop state.

- [x] **Rev Task 6: Conditional old experiments**

  Re-open table-size variants, unstuff-once, or the fused direct-band kernel
  only if the stop rule leaves a material instruction gap and the exact
  profile identifies the corresponding cost. Inspect the fused-kernel branch's
  refill implementation before any rerun.

  Stop rule: final candidates are 1.38-1.42x FlashRaw. The remaining old
  experiments were not rerun because the exact profile no longer supports the
  old refill-dependent assumptions strongly enough to justify another stack.

## Register-pressure baseline

The sample loop already keeps `FastBitReader.cache`, `buffered_bits`, and
`position` in registers, but unrelated state forces repeated stack/table
loads:

- `49c934 mov 0x18(%rsp),%r13` reloads a long-lived iterator/table-owner
  value once per fast hit. `%r13` is needed for the cache, so the captured
  value cannot stay live across the sample.
- `49c906 mov 0x18(%r8),%rax` and `49c913 mov 0x10(%r8),%rax` reload the
  fast-table length and data pointer every sample. They remain because
  `fast_lookup` is a bounds-checked boxed slice reached through the component
  table reference.
- `49c734 mov (%rdx),%r8` reloads the current table reference. The component
  iterator pointer is kept in `%rdx`, while `%r8` is reused by the inlined
  refill/table code.
- `49c71c`, `49c72a`, `49cc5c`, and `49cc6d` reload current scratch
  length/data pointers from the stack for Vec indexing twice per sample.
- `49cc36 and 0x104(%rsp),%r12d` reads the precision mask from the stack for
  every sample.
- `49cc77` and `49cc7c` read the point-transform flag and shifted-buffer
  pointer despite this input using point transform zero.
- Loop state is repeatedly stored/reloaded around `0x2f0`, `0x2e8`, `0x2f8`,
  and `0x300` on the stack, including row/component iterators.

The complete per-PC evidence and annotations are in
`step0-exact-categorized.tsv`, `step0-exact-annotate-outside-lookup.txt`, and
`step0-register-pressure.txt`.

---

## Superseded draft task details

The sections below preserve the original proposed ordering for review history.
They are not authoritative after the exact-count revision above.

## Task 0: Baseline and Confirmation

**Files:**

- Create: `docs/reference/decoders/perf/dng-gap-profiles/step0-profile.txt`
- Create: `docs/reference/decoders/perf/dng-gap-profiles/step0-annotate.txt`
- Create: `docs/reference/decoders/perf/dng-gap-profiles/step0-reader-counters.txt`
- Temporarily modify, then restore before commit:
  `crates/fornax-codec/src/ljpeg.rs`

**Interfaces:**

- Consumes: baseline commit `65cbb38`, `fornax-codec::ljpeg`, and the existing
  `dng_gap_probe`.
- Produces: source-line instruction shares, stack/register observations for
  `FastBitReader`, and 5D3/Pixel 8 fast-lookup miss and slow-path byte counts.

- [x] **Step 1: Build the baseline profile target**

Run:

```bash
CARGO_PROFILE_RELEASE_DEBUG=line-tables-only \
  cargo build -p fornax-raw-decoder --release --bench dng_gap_probe
```

Expected: the bench binary is under `target/release/deps/dng_gap_probe-*` and
contains line tables.

- [x] **Step 2: Capture and annotate the 5D3 1T predictor-1 closure**

Run the 5D3 LJPEG input with one Rayon worker under `perf record`, then save
`perf annotate --stdio -l` output. Group instruction samples into refill
(`fill_to`, `next_byte`), marker/bounds checks, table lookup, difference
extraction, predictor load/store (`scratch.current` and
`scratch.previous`), `apply_difference`, and loop overhead. List every
panic/bounds-check branch remaining in the hot loop.

Input:

```text
external/raw.pixls.us/Adobe DNG Converter/Canon EOS 5D Mark III/5G4A9394-compressed-lossless.DNG
```

Expected: `step0-annotate.txt` contains the source-line table and branch list,
and `step0-profile.txt` records the exact commands, binary, event/sample
period, sample count, lost events, and summary shares.

- [x] **Step 3: Inspect reader state placement**

Inspect the annotated assembly or use `cargo asm` if available. Determine
whether `FastBitReader.cache`, `buffered_bits`, and `position` remain in
registers or are spilled per sample. Quote the relevant instruction lines and
record a yes/no plus source/asm locations in `step0-profile.txt`.

Expected: Step 2 proceeds only if spills and line attribution justify it.

- [x] **Step 4: Add temporary counters**

Without committing them, count:

```text
fast_lookup_full_false / fast_lookup_calls
slow_path_bytes_refilled / samples
```

Run on 5D3 and Pixel 8 at one thread. Save the raw output and method in
`step0-reader-counters.txt`, then restore the production source exactly.

Pixel 8 input:

```text
external/raw.pixls.us/Google/Pixel 8 Pro/PXL_20240415_103400204.RAW-02.ORIGINAL.dng
```

Expected: both inputs have measured miss rates and slow-path bytes per sample;
the worktree is clean except for Step 0 artifacts.

- [x] **Step 5: Commit the profile artifacts**

```bash
git add dng-gap-profiles docs/plans/chore-ljpeg-instruction-gap.md
git commit -m "docs: profile lossless JPEG instruction costs"
```

---

## Task 1: Word Refill

**Files:**

- Modify: `crates/fornax-codec/src/ljpeg.rs`
- Create/test: `docs/reference/decoders/perf/dng-gap-profiles/step1-lookup.txt`

**Interfaces:**

- Consumes: `FastBitReader`, `HuffmanTable::decode_difference_fast`, and the
  Step 0 miss/counter evidence.
- Produces: one refill per sample before lookup, with a checked eight-byte
  big-endian fast load and cold marker/stuffing/end fallback.

- [ ] **Step 1: Add failing reader tests first**

Add tests for:

- The existing reference-versus-fast reader test.
- Inserting `ff 00` every several byte periods and comparing all consumed
  bits against `BitReader`.
- Exact marker termination and zero fill after the marker.
- Truncated streams shorter than eight bytes and ending inside a potential
  word.
- A 16-bit Huffman code.
- Leftover lookahead bits followed by the slow path, proving the cache
  composition does not duplicate or reorder bytes.

Run:

```bash
cargo test -p fornax-codec --features lossless-jpeg ljpeg
```

Expected: at least one new behavioral test fails against the old refill shape
before production code changes.

- [ ] **Step 2: Implement the fast word refill**

When `buffered_bits < 32`, load a big-endian `u64` at `position` through
`first_chunk::<8>()`, checking availability once. Detect any `0xff` byte with:

```text
(!w).wrapping_sub(0x0101_0101_0101_0101)
    & w
    & 0x8080_8080_8080_8080
```

If no byte is `0xff`, OR `w >> buffered_bits` into the cache, set
`taken = (63 - buffered_bits) >> 3`, advance `position` by `taken`, and add
`taken * 8` buffered bits. Otherwise call a `#[cold] #[inline(never)]` helper
that preserves the current byte loop, including `ff 00` unstuffing, marker
termination, and zero fill after end. Use `usize` bit counts.

Call refill once per sample before table lookup. Remove the extra `fill_to`
calls from `decode_fast` and `decode_difference_fast`; code bits plus amplitude
must come from the same refill.

- [ ] **Step 3: Run focused correctness tests**

```bash
cargo test -p fornax-codec --features lossless-jpeg ljpeg
cargo bench -p fornax-codec --features lossless-jpeg \
  --bench ljpeg_gap_probe -- --self-check
```

Expected: all focused tests and the synthetic generator self-check pass.

- [ ] **Step 4: Run the complete codec tests and equality gate**

```bash
cargo test -p fornax-codec --all-features
RAYON_NUM_THREADS=1 \
  target/release/deps/dng_gap_probe-* equality \
  "external/raw.pixls.us/Adobe DNG Converter/Canon EOS 5D Mark III/5G4A9394-compressed-lossless.DNG"
RAYON_NUM_THREADS=1 \
  target/release/deps/dng_gap_probe-* equality \
  "external/raw.pixls.us/Google/Pixel 8 Pro/PXL_20240415_103400204.RAW-02.ORIGINAL.dng"
```

Expected: codec tests pass and both files print exact three-way equality.

- [ ] **Step 5: Measure all required cases**

Build from the same baseline commit and this candidate. For each case run
three fresh interleaved processes, five rounds per process; alternate Fornax
and FlashRaw in one session. Capture instructions, branches, branch misses,
cycles, wall time, IPC, and instructions/sample. Include 5D3 and Pixel 8 at
1T/15T, iPhone SE at 1T, synthetic smooth/noisy one/two-component scans, and
same-session FlashRaw.

- [ ] **Step 6: Keep or revert**

Keep only if the gain exceeds the full measured noise spread without a
regression in another required case. Otherwise record the numbers and run:

```bash
git revert --no-edit <candidate-commit>
```

Commit the artifact and decision in the final kept or revert commit. Report
the measured miss rate and slow-path bytes per sample for this candidate.

---

## Task 2: Reader State in Registers

**Files:**

- Modify: `crates/fornax-codec/src/ljpeg.rs`
- Create/test: `docs/reference/decoders/perf/dng-gap-profiles/step2-loop-predictor.txt`

**Interfaces:**

- Consumes: Step 0 spill result and the retained Task 1 reader.
- Produces: hot-loop locals for `cache`, `buffered_bits`, and `position`, with
  cold miss/error/marker code taking and returning state by value.

- [ ] **Step 1: Re-check the Step 0 spill evidence**

Proceed only if Step 0 found per-sample loads/stores. Otherwise record the
observed registers and mark Step 2 not applicable.

- [ ] **Step 2: Add a failing state-equivalence test**

Construct reads that cross in-order byte refills, stuffing, marker end,
truncation, and a recorded Huffman error. Assert that round-tripping reader
locals through the restructured path produces exactly the same bits, position,
end state, and first error as `&mut FastBitReader`.

- [ ] **Step 3: Restructure the hot loop**

Hold `cache`, `buffered_bits`, and `position` as locals in
`decode_scan_rows_with_scratch`. Use `#[cold] #[inline(never)]` helpers that
accept and return those values for the miss, marker/end, and error-recording
paths. Do not move table lookup or amplitude extraction to a colder path.

- [ ] **Step 4: Re-check asm and behavior**

Repeat the source-line annotation and show which reader fields no longer spill.
Run codec tests, synthetic self-check, and strict 5D3/Pixel 8 equality.

- [ ] **Step 5: Measure and keep or revert**

Use the full Task 1 protocol and same-session FlashRaw. Keep only outside the
measured noise spread. Revert a rejected candidate in its own commit and save
all results in `step2-reader-registers.txt`.

---

## Task 3: Inner-Loop Overhead

**Files:**

- Modify: `crates/fornax-codec/src/ljpeg.rs`
- Create/test: `docs/reference/decoders/perf/dng-gap-profiles/step3-refill.txt`

**Interfaces:**

- Consumes: retained reader from Tasks 1-2.
- Produces: up to three independently measured loop-overhead candidates.

- [ ] **Step 1: Add failing equivalence tests**

For 3a, cover first sample, first column on later rows, first row after the
initial pixel, nonzero point transform, and one/two components. For 3b, compare
a local-predictor/chunk-writing kernel against the row decoder for packed and
unpacked outputs. For 3c, cover maximum precision and all peek counts.

Expected: each new test fails for the missing candidate shape before the
corresponding production edit.

- [ ] **Step 2a: Measure boundary peeling and point-transform loops separately**

Hoist `row == 0`, `column == 0`, and `point_transform != 0` out of the
per-sample loop by peeling the initial sample/row/first columns and using
separate transform and no-transform loops. Commit this candidate alone, measure
under the full protocol, and keep or revert it in its own commit.

- [ ] **Step 2b: Measure local predictors and chunked row writes**

Starting from the kept 3a state or the last kept baseline state, keep each
component's predictor in a local and write the row with
`chunks_exact_mut(components)`, avoiding repeated index arithmetic and
`scratch.current[index - components]`. Commit alone, measure, and keep or
revert in its own commit.

- [ ] **Step 2c: Measure conversion and mask cleanup**

Starting from the preceding kept state, replace `u16::try_from(...).expect(...)`
in `apply_difference` and `peek_no_fill` with checked/truncating const helpers
in the crate's existing style, and drop redundant masking when
`cache >> (64 - count)` already yields exactly `count` bits. Commit alone,
measure, and keep or revert in its own commit.

- [ ] **Step 3: Validate each retained candidate**

After each keep, run focused tests, complete codec tests, synthetic
self-check, and strict 5D3/Pixel 8 equality. Save per-candidate before/after
instructions/sample, IPC, branch misses, wall time, 1T/15T ratios, and the
decision in `step3-inner-loop.txt`.

---

## Task 4: Table Design

**Files:**

- Modify: `crates/fornax-codec/src/ljpeg.rs`
- Create/test: `docs/reference/decoders/perf/dng-gap-profiles/step4-reader-registers.txt`

**Interfaces:**

- Consumes: retained Steps 1-3 and Step 0c miss-rate data.
- Produces: independent measurements of 12-bit fused, 11-bit fused, and
  two-level 9-10-bit code tables.

- [ ] **Step 1: Record baseline table bytes and miss rate**

Record bytes per DHT tile for the 128 KiB 16-bit lookup, 13-bit fast lookup,
and total, plus the current fast miss rate. Stop table redesign early if the
prior steps moved the profile so misses are no longer a material cost, and
record that ruling.

- [ ] **Step 2: Test a 12-bit fused table alone**

Commit the candidate alone, run focused tests and full equality, record table
bytes/tile and Table 4 measurements under the full protocol, then keep or
revert in its own commit.

- [ ] **Step 3: Test an 11-bit fused table alone**

From the preceding kept state, repeat the same independent test, measurement,
and keep/revert process.

- [ ] **Step 4: Test a two-level 9-10-bit code table alone**

Use a small code table containing length and symbol, extract the amplitude
directly from the cache, and remove the 128 KiB 16-bit lookup and miss path for
the supported shape. Preserve invalid, 16-bit, malformed, and truncated
behavior. Measure and keep or revert alone.

- [ ] **Step 5: Record table footprint and outcome**

`step4-table-design.txt` must contain bytes per tile, observed miss rate,
instructions/sample, IPC, branch misses, wall time, 1T/15T same-session
FlashRaw ratios, and a keep/revert reason for every table candidate.

---

## Task 5: Conditional Re-evaluation

**Files:**

- Modify: `crates/fornax-codec/src/ljpeg.rs` only for a justified candidate
- Create/test: `docs/reference/decoders/perf/dng-gap-profiles/step5-conditional.txt`

**Interfaces:**

- Consumes: final profile after Steps 1-4 and historical prototype branches.
- Produces: either a measured re-evaluation of the three named old ideas or an
  evidence-backed decision not to rerun them.

- [ ] **Step 1: Apply the stop rule**

Stop before Step 5 when instructions/sample is within approximately 1.3x
FlashRaw or when the last two attempted steps produced no gain beyond noise.
Record the exact stop-rule comparison in `step5-revaluation.txt`.

- [ ] **Step 2: Inspect the historical fused-kernel branch first**

Read the fused direct-band prototype branch/source and report whether it reused
the old byte-wise `fill_to`. Do not benchmark it when the final profile does not
justify re-evaluation.

- [ ] **Step 3: Re-test only justified ideas, one at a time**

If justified, test unstuff-once, const-generic one/two-component loops, and the
fused direct-band kernel separately under the full protocol. Each candidate is
one commit and each rejection is reverted in its own commit.

---

## Task 6: Final Report and Validation

**Files:**

- Modify: `docs/reference/decoders/perf/dng-remaining-gap.md`
- Create: `docs/reference/decoders/perf/dng-gap-profiles/ljpeg-instruction-hypotheses.txt`
- Update: all `docs/reference/decoders/perf/dng-gap-profiles/*` artifacts

**Interfaces:**

- Consumes: every Step 0-5 artifact and decision.
- Produces: the final host-specific report and reproducible evidence trail.

- [ ] **Step 1: Write the final report**

Include the Step 0 source-line instruction table; per-step before/after
instructions/sample, IPC, branch misses, wall time, 1T/15T, and same-session
FlashRaw ratios; the kept/reverted table with reasons; Step 0c miss rates and
slow bytes; final asm spills and remaining bounds checks; stop-rule result; and
any disagreement with the source review as an explicit finding.

- [ ] **Step 2: Record environment and limitations**

State host CPU, visible CPU count, WSL/kernel or native kernel, glibc, THP and
THP defrag, release profile, page-cache state, and that all numbers are
host-specific.

- [ ] **Step 3: Run final validation**

Run:

```bash
cargo test -p fornax-codec --all-features
cargo bench -p fornax-codec --features lossless-jpeg \
  --bench ljpeg_gap_probe -- --self-check
scripts/agent-validation.sh
```

Run strict equality for 5D3, Pixel 8, and iPhone SE. Record commands and
outcomes, including any skipped or unavailable check, in the report and pull
request description.

- [ ] **Step 4: Inspect and commit**

Inspect the complete diff and `git log --oneline` for one baseline profile
commit, one candidate/revert pair per experiment, and the final report commit.
Commit the report and artifacts, then push and open one semantic pull request
against `main`.

## Validation

- `cargo test -p fornax-codec --all-features`
- `cargo bench -p fornax-codec --features lossless-jpeg --bench
  ljpeg_gap_probe -- --self-check`
- Strict `dng_gap_probe equality` for 5D3, Pixel 8, and iPhone SE
- Three fresh interleaved processes x five rounds for every measured case
- `scripts/agent-validation.sh`
- `git diff --check`

## Decisions and Notes

- PR #734 is already merged as baseline commit `65cbb38`; the worktree starts
  from `origin/main` at that commit.
- Existing reverted table data remains historical evidence only. Table sizes
  must be re-measured after refill changes.
- The prior table experiment measured a 128 KiB 16-bit lookup plus a 32 KiB
  13-bit fast lookup, or 163,840 bytes per DHT tile, but the new direct-band
  row path may allocate those tables per tile. Step 4 must confirm current
  lifetime and footprint rather than assuming the historical cache behavior.
- Measurement artifacts belong in
  `docs/reference/decoders/perf/dng-gap-profiles/`, beside the existing DNG
  performance references.
- A source-review claim remains a hypothesis until Step 0 or a candidate
  measurement confirms it.
- Step 0 found 1.445842% fast misses on 5D3 and 1.316251% on Pixel 8, with
  0.841867 and 0.768870 slow-path bytes per sample. Reader fields are
  register-resident on the dominant path; Step 2 remains conditional on
  retained Step 1 assembly.
- Native `perf annotate` segfaulted with the available GNU objdump, so Step 0
  used sampled IPs, DWARF `addr2line`, and annotated disassembly for equivalent
  source attribution.
