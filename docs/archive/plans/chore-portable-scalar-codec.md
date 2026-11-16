> Status: archived completed plan.
> Merged: #734 `chore/portable-scalar-codec` on 2026-11-15.
> Current guidance: [current documentation](../../reference/decoders/dng-remaining-gap.md).

# Portable Scalar DNG Codec Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Remove OS-specific output allocation from `fornax-codec`, quantify
the resulting regression, and retain only portable scalar improvements that
beat the same-session baseline outside noise while preserving exact decode
output.

**Architecture:** The work starts with one revertible commit that replaces the
Linux huge-page allocator with ordinary `Vec` allocations. A regression artifact
then records with/without-policy timings and fault counts against same-session
FlashRaw. Portable LJPEG hypotheses are tested independently, with only measured
winners kept. A caller-owned-output experiment remains bench-only or additive
behind an API decision; SIMD, target features, platform APIs, and allocator
tricks are out of scope.

**Tech Stack:** Rust workspace, `fornax-codec`, `fornax-raw-decoder` release
benches, Rayon, `perf stat`, `perf annotate`, existing FlashRaw 0.2.0 adapter.

**Spec:** User request in the current task, plus
`docs/reference/decoders/dng-remaining-gap.md` and PR #728.

## Global Constraints

- No `cfg(target_os)`, `libc`, `madvise`, `mmap` advice, THP assumptions, or
  allocator tricks in `fornax-codec`.
- Plain `Vec` and standard library only; do not add a dependency.
- Do not add SIMD, intrinsics, `portable_simd`, or `target_feature` code.
- Single thread, release, page-cache-hot, and the reported host settings unless
  a case explicitly measures 15 threads.
- Every timing comparison uses fresh interleaved processes, enough rounds to
  show noise, and same-session FlashRaw.
- Every retained decode change passes the three-way dimensions, storage type,
  bits, and pixels gate.
- One hypothesis per change. Revert any wall-time change inside noise.
- Do not touch decode behavior unrelated to a measured hypothesis.
- Do not merge a public API change. Stop and ask for an API decision before
  exposing decode-into.
- Host-specific numbers are not portable performance claims.

## Scope

In scope:

- Removing the Linux allocator and its allocator-only tests.
- Portable and cross-target checks for `fornax-codec`.
- With-policy versus without-policy regression measurements.
- LJPEG Huffman-size, fallback, and table-reuse experiments.
- A non-public caller-owned-output prototype.
- Measurement and a separate follow-up branch for a scalar fused kernel.
- Bounds-check/profile experiments only where measurements justify them.
- Deflate re-evaluation only after the higher-priority work is complete.

Out of scope:

- SIMD and target-specific code.
- Changing owned-return decode APIs without approval.
- Changing workspace profiles for downstream users without approval.
- Portable performance guarantees.

## Files Expected to Change

- `crates/fornax-codec/src/samples.rs`
- `crates/fornax-codec/src/deflate.rs`
- `crates/fornax-codec/src/jpeg.rs`
- `crates/fornax-codec/src/jpegxl.rs`
- `crates/fornax-codec/src/ljpeg.rs`
- `crates/fornax-codec/src/lib.rs`
- `crates/fornax-codec/Cargo.toml`
- `Cargo.lock`
- `crates/fornax-codec/benches/ljpeg_gap_probe.rs`
- `crates/fornax-raw-decoder/benches/dng_gap_probe/linux.rs`
- `docs/reference/decoders/dng-remaining-gap.md`
- `docs/reference/decoders/dng-gap-profiles/*`
- `docs/plans/chore-portable-scalar-codec.md`

## Review Focus

- Large U8/U16/U32/F32/F64 decodes still return the same dimensions, storage,
  bits, and pixels after the allocator is removed.
- The allocator-only tests disappear without deleting tests for codec behavior.
- Empty and exceptional allocation paths still behave like ordinary `Vec`.
- Huffman table experiments handle invalid, long, and missing codes exactly as
  the baseline does.
- Bench-only APIs do not accidentally become public or production dispatch.
- Fifteen-thread changes do not hide single-thread regressions or reorder tiles.

---

### Task 1: Remove the platform allocator in one commit

**Files:**
- Modify: `crates/fornax-codec/src/samples.rs`
- Modify: `crates/fornax-codec/src/deflate.rs`
- Modify: `crates/fornax-codec/src/jpeg.rs`
- Modify: `crates/fornax-codec/src/jpegxl.rs`
- Modify: `crates/fornax-codec/src/ljpeg.rs`
- Modify: `crates/fornax-codec/Cargo.toml`
- Modify: `Cargo.lock`
- Test: existing `fornax-codec` decode contracts and real-file equality probe

**Interfaces:**
- Consumes: current owned-return `decode` APIs.
- Produces: identical owned `SampleBuffer` outputs allocated by `vec![zero; len]`.

- [ ] **Step 1: Record the pre-removal source and passing baseline**

Run:

```bash
cargo test -p fornax-codec --all-features
rg -n 'zeroed_samples|cfg\\(target_os|libc|madvise|MADV|alloc_zeroed' \
  crates/fornax-codec
```

Expected: tests pass and the output lists every platform-only allocator site.

- [ ] **Step 2: Replace every large-output helper call with plain zeroed `Vec`**

Delete `zeroed_samples`, `hugepage_zeroed_samples`, `advise_hugepage`, their
constants, unsafe allocation code, and allocator-only tests. Use the original
element types at each call site (`0`, `0_u16`, or `0.0_f32`) rather than type
inference through a helper.

- [ ] **Step 3: Remove the target-specific `libc` dependency**

Remove the `fornax-codec` target dependency and update `Cargo.lock` only through
Cargo.

- [ ] **Step 4: Verify portability and behavior**

Run:

```bash
rg -n 'cfg\\(target_os|libc|madvise|MADV|alloc_zeroed' crates/fornax-codec
cargo test -p fornax-codec --all-features
cargo check -p fornax-codec --target x86_64-pc-windows-msvc --all-features
cargo check -p fornax-codec --target aarch64-apple-darwin --all-features
```

Expected: the grep has no matches; tests pass; both target checks pass. If a
target is absent, install it and rerun rather than claiming the check.

- [ ] **Step 5: Run the real-file equality gate**

Run the probe equality mode for 5D3 uncompressed, 5D3 LJPEG, iPhone SE LJPEG,
Pixel 8 LJPEG, FP16 Deflate, and FP32 Deflate.

Expected: every case prints matching dimensions, storage, bits, and pixels.

- [ ] **Step 6: Commit**

```bash
git add Cargo.lock crates/fornax-codec docs/plans/chore-portable-scalar-codec.md
git commit -m "refactor: remove platform output allocation"
```

### Task 2: Measure and document the allocator regression

**Files:**
- Modify: `crates/fornax-raw-decoder/benches/dng_gap_probe/linux.rs` only if a
  measurement helper is missing
- Modify: `docs/reference/decoders/dng-remaining-gap.md`
- Create: `docs/reference/decoders/dng-gap-profiles/portable-allocation-regression.txt`

**Interfaces:**
- Consumes: Task 1's portable allocator.
- Produces: a measured baseline for every later hypothesis.

- [ ] **Step 1: Build the release probe and warm every input**

```bash
cargo build -p fornax-raw-decoder --release --bench dng_gap_probe
```

- [ ] **Step 2: Capture with-policy timings**

Use the already committed allocator state from `origin/main` or a temporary
worktree built from `aa1de4e`. For each measured headline file, alternate
Fornax and FlashRaw in fresh processes, three times per decoder, at one and
fifteen workers, with the established warmup/round count.

- [ ] **Step 3: Capture without-policy timings**

Repeat the same matrix on this branch after Task 1.

- [ ] **Step 4: Capture minor faults**

Run one page-cache-hot decode under:

```bash
perf stat -e minor-faults,page-faults,instructions,time
```

for both allocator states, both decoders, and both thread counts. Record the
full commands and raw values.

- [ ] **Step 5: Write the regression artifact**

Include wall medians and observed ranges, minor faults, ms and percent deltas,
same-session FlashRaw ratios, equality-gate status, host, kernel, glibc, THP,
and the explicit non-portability caveat.

- [ ] **Step 6: Update the report**

State exactly which cases lose FlashRaw parity after removing the policy and
which remain at parity.

- [ ] **Step 7: Commit**

```bash
git add docs/reference/decoders
git commit -m "docs: report portable allocation regression"
```

### Task 3A: Test smaller fast Huffman tables

**Files:**
- Modify: `crates/fornax-codec/src/ljpeg.rs`
- Modify: `crates/fornax-codec/benches/ljpeg_gap_probe.rs`
- Create: `docs/reference/decoders/dng-gap-profiles/ljpeg-table-experiments.txt`

**Interfaces:**
- Consumes: Task 2's without-policy baseline.
- Produces: measured table footprint and decode cost for 11-, 12-, and 13-bit
  fast tables.

- [ ] **Step 1: Add table-footprint reporting to the synthetic probe**

Report exact retained lookup and fast-lookup bytes per parsed tile.

- [ ] **Step 2: Run the unchanged 13-bit baseline**

Measure smooth/noisy one/two-component synthetic scans and real 5D3, iPhone SE,
and Pixel 8 at one and fifteen threads.

- [ ] **Step 3: Change only the fast lookahead constant to 12 bits**

Update dependent tests and run the full codec suite. Measure the same matrix.

- [ ] **Step 4: Keep or revert 12 bits**

Keep only if real-file wall time improves outside noise and equality remains
exact. Record instructions/sample and `parse_jpeg` share changes.

- [ ] **Step 5: Repeat the isolated experiment at 11 bits**

Use the same tests and matrix, comparing 11 bits against the retained best
state. Revert unless it independently wins outside noise.

- [ ] **Step 6: Commit only the winner**

```bash
git add crates/fornax-codec docs/reference/decoders
git commit -m "perf: tune lossless JPEG lookup tables"
```

### Task 3B: Test canonical-code fallback without the 16-bit table

**Files:**
- Modify: `crates/fornax-codec/src/ljpeg.rs`
- Modify: `crates/fornax-codec/benches/ljpeg_gap_probe.rs`
- Modify: `docs/reference/decoders/dng-gap-profiles/ljpeg-table-experiments.txt`

**Interfaces:**
- Consumes: Task 3A's retained state.
- Produces: either a smaller fast table with a slow long-code path or evidence
  that the fallback costs more than it saves.

- [ ] **Step 1: Add tests that force a long Huffman code**

Prove exact symbol decode and malformed-code behavior before changing storage.

- [ ] **Step 2: Replace the 16-bit table with per-length canonical bounds**

Keep the fast lookup for common codes and resolve misses through a canonical
slow path. Do not change error reporting.

- [ ] **Step 3: Run the focused and full codec tests**

Expected: fast and general scans remain byte-for-byte equal, including the
forced long-code case.

- [ ] **Step 4: Measure synthetic and real files**

Record table bytes, instructions/sample, `parse_jpeg`, and wall medians at one
and fifteen threads.

- [ ] **Step 5: Keep or revert**

Revert if any real file regresses outside noise or if the gain is confined to
table-construction counters without a wall-time win.

- [ ] **Step 6: Commit only the winner**

```bash
git add crates/fornax-codec docs/reference/decoders
git commit -m "perf: resolve long lossless JPEG codes canonically"
```

### Task 3C: Test plain shared Huffman tables across identical DHTs

**Files:**
- Modify: `crates/fornax-codec/src/ljpeg.rs`
- Modify: `crates/fornax-codec/benches/ljpeg_gap_probe.rs`
- Modify: `docs/reference/decoders/dng-gap-profiles/ljpeg-table-experiments.txt`

**Interfaces:**
- Consumes: Task 3A/3B's retained state.
- Produces: a portable `Arc`-based cache keyed by the exact DHT definition.

- [ ] **Step 1: Add a deterministic test with repeated DHT bytes**

Assert the parsed tables decode identically to independently parsed tables.

- [ ] **Step 2: Add an `Arc` cache keyed by `(counts, symbols)`**

Build one immutable table per distinct DHT and share it across parsed tiles
within the decode request. Do not use threads, globals, or platform APIs.

- [ ] **Step 3: Measure the cache**

Reuse must eliminate table construction samples and table bytes per additional
identical tile. Record hit rate, minor faults, instructions/sample, and wall
time.

- [ ] **Step 4: Keep or revert**

Keep only if both one-thread and fifteen-thread real-file results stay outside
noise on the winning side. Revert if `Arc` lookup or allocation dominates.

- [ ] **Step 5: Commit only the winner**

```bash
git add crates/fornax-codec docs/reference/decoders
git commit -m "perf: share lossless JPEG Huffman tables"
```

### Task 4: Prototype caller-owned output without an API change

**Files:**
- Modify: `crates/fornax-codec/benches/ljpeg_gap_probe.rs`
- Create: `docs/reference/decoders/dng-gap-profiles/ljpeg-reused-output.txt`

**Interfaces:**
- Consumes: retained LJPEG decoder internals.
- Produces: bench-only evidence and a proposed additive API shape.

- [ ] **Step 1: Add a bench-only decode loop using one caller-provided buffer**

Decode several copies of the same synthetic and real tile payload into a
pre-touched `Vec<u16>`, clearing/reusing it without per-decode allocation.

- [ ] **Step 2: Compare against fresh owned outputs**

Measure one and fifteen threads, minor faults, and wall time with enough
rounds for noise.

- [ ] **Step 3: Document the potential API**

Describe ownership, error behavior on partial output, dimensions/overwrite
semantics, and a safe Rust signature. Report API impact and safety implications.

- [ ] **Step 4: Stop for approval**

Do not expose or merge a public decode-into API. Present the numbers and wait
for an API decision.

### Task 5: Evaluate a fused scalar LJPEG kernel on a separate branch

**Files:**
- Branch: `chore/portable-ljpeg-fused-kernel`
- Modify: `crates/fornax-codec/src/ljpeg.rs`
- Modify: `crates/fornax-codec/benches/ljpeg_gap_probe.rs`
- Create: `docs/reference/decoders/dng-gap-profiles/ljpeg-fused-scalar.txt`

**Interfaces:**
- Consumes: Task 2 baseline and Task 3 retained table representation.
- Produces: a scalar-only kernel or a measured rejection.

- [ ] **Step 1: Capture cycles, IPC, and instructions/sample**

Measure Fornax and FlashRaw on 5D3, iPhone SE, and Pixel 8. Use the existing
88% predictor-1 annotation to set per-category instruction budgets before code.

- [ ] **Step 2: Implement only the budgeted kernel**

Use const-generic components, local bit state and predictors, multi-symbol
refill, pre-unstuffed scan with a cold 0xff path, branchless sign extension,
buffered errors, and row iteration. No SIMD or target features.

- [ ] **Step 3: Verify exact output**

Run forced-stuffing, malformed-code, synthetic one/two-component, full codec,
and real-file three-way equality tests.

- [ ] **Step 4: Measure against the portable baseline**

Success requires 5D3 one-thread to improve by at least 15% outside noise, no
iPhone SE or Pixel 8 regression, and lower instructions/sample. Restate the
original 130 ms target relative to the measured without-policy baseline.

- [ ] **Step 5: Keep or reject**

Commit the kernel only on its separate branch if it meets the bar. Otherwise
record the negative result and revert it.

### Task 6: Bounds checks and build profiles

**Files:**
- Modify: `crates/fornax-codec/src/ljpeg.rs` only if assembly shows removable
  checks
- Modify: `docs/reference/decoders/dng-gap-profiles/ljpeg-fused-scalar.txt`

**Interfaces:**
- Consumes: current scalar hot loops.
- Produces: assembly evidence and a profile recommendation.

- [ ] **Step 1: Inspect release assembly for bounds checks and panic calls**

Attribute remaining checks to row samples, Huffman indexes, and bit-reader
loads.

- [ ] **Step 2: Try one layout change at a time**

Use row slices checked once or `chunks_exact` only where annotation shows a
check in the hot instruction stream. Benchmark and revert within noise.

- [ ] **Step 3: Test bench-only profile settings**

Report LTO, codegen-units, opt-level, and panic strategy effects using command
line overrides. Do not change workspace profiles without approval.

### Task 7: Fifteen-thread LJPEG scaling

**Files:**
- Modify: `crates/fornax-codec/src/ljpeg.rs` only if a measured scheduling
  change is justified
- Modify: `docs/reference/decoders/dng-gap-profiles/portable-scalar.txt`

**Interfaces:**
- Consumes: retained portable decoder.
- Produces: per-file scaling efficiency and per-tile temporary attribution.

- [ ] **Step 1: Measure one-thread and fifteen-thread scaling**

Record wall time, speedup, `minor-faults`, instructions, and tile count.
- [ ] **Step 2: Attribute shared state and temporary cost**

Separate output, Huffman tables, scan buffers, Rayon scheduling, and source
mapping.
- [ ] **Step 3: Change scheduling only if evidence identifies a bottleneck**

Test one scheduling hypothesis, then keep or revert using the same gate.

### Task 8: Re-evaluate Deflate after Tasks 1-7

**Files:**
- Modify: `crates/fornax-codec/src/deflate.rs` only if a portable hypothesis is
  supported
- Modify: `docs/reference/decoders/dng-remaining-gap.md`

**Interfaces:**
- Consumes: portable allocation regression.
- Produces: final Deflate recommendation.

- [ ] **Step 1: Re-profile without platform allocation**

Capture wall time, faults, instructions, and current miniz versus zlib-rs data.
- [ ] **Step 2: Test at most one backend hypothesis**

Keep it only if FP16 and FP32 improve outside noise at both thread counts and
the equality gate remains green.
- [ ] **Step 3: Revert if the effect is within noise or benefits only one case**

### Task 9: Final report and validation

**Files:**
- Modify: `docs/reference/decoders/dng-remaining-gap.md`
- Create: `docs/reference/decoders/dng-gap-profiles/portable-scalar.txt`

**Interfaces:**
- Consumes: every measurement and decision above.
- Produces: the final before/after table and conclusion.

- [ ] **Step 1: Write one kept/reverted experiment table**

Include hypothesis, files, 1T/15T before/after, minor faults,
instructions/sample, decision, and reason.
- [ ] **Step 2: Add per-file results and parity status**

State which cases lose FlashRaw parity and which portable changes recover it.
- [ ] **Step 3: State the scalar limit**

Summarize instruction-bound and latency-bound evidence and what scalar-only
portable code can and cannot reach.
- [ ] **Step 4: Record the environment**

Host, kernel, glibc, THP settings, release mode, cache state, thread counts, and
the non-portability caveat.
- [ ] **Step 5: Run final verification**

```bash
cargo test -p fornax-codec --all-features
cargo test -p fornax-raw-decoder
rg -n 'cfg\\(target_os|libc|madvise|MADV|alloc_zeroed|portable_simd|target_feature' \
  crates/fornax-codec
scripts/agent-validation.sh
git diff --check
```

Expected: all tests and validation pass; the grep has no portability-policy
violations.

## Decisions and Notes

- Use the primary checkout's materialized `external/raw.pixls.us` files as
  read-only benchmark inputs; do not copy or commit RAW payloads.
- The existing `dng_gap_probe` is Linux-only for profiling and process controls.
  The codec itself must remain portable; probe-only OS code may stay outside
  `fornax-codec`.
- Apply only wins outside observed noise at both thread counts. A statistically
  detectable but operationally tiny synthetic-only win is recorded and
  reverted unless it also wins on a real file.
- Public API changes stop after the prototype.
