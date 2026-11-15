# LJPEG Critical-Path Plan

> **For agentic workers:** Use `superpowers:executing-plans` to implement this
> plan task-by-task. Keep every candidate isolated until its measurements and
> correctness gates are complete.

**Goal:** Determine whether the remaining loop-carried bit-reader latency or
branch behavior in Fornax's portable scalar LJPEG decoder contains a removable
cost, test at most two bounded changes, and record whether the decoder is at
its practical scalar floor on this host.

**Architecture:** The baseline is the merged portable scalar implementation
from PR #736. First profile the existing `DecodeRowSampler` loop and exact
assembly without changing production code. Then test, one at a time, a
register-resident bit-reader row loop and a branchless difference decoder. Keep
a candidate only when it improves real-file wall time beyond the observed
noise spread and all correctness gates pass.

**Tech Stack:** Rust 2024, `fornax-codec`, `ljpeg_gap_probe`, `dng_gap_probe`,
Rayon, `perf stat`, `perf record`, and the existing FlashRaw and Rawler
benchmark adapters.

## Global Constraints

- Use portable scalar Rust only. Do not add `cfg(target_os)`, `libc`,
  `madvise`, SIMD, intrinsics, `portable_simd`, or `target_feature`.
- Do not add dependencies. Keep the owned-return public API unchanged.
- Do not use `as` casts if clippy rejects them; use small truncation helpers.
- Change one production hypothesis at a time. Never stack an unmeasured change
  on another candidate.
- Use three fresh processes and five interleaved rounds per process. Report
  median and min-max wall time.
- Measure 5D3 and Pixel 8 at one and fifteen threads, iPhone SE at one thread,
  and one- and two-component smooth and noisy synthetic scans.
- Run FlashRaw in the same measurement session. Never compare ratios across
  sessions.
- Keep a candidate only if real-file wall time beats the measured noise spread,
  with the exact three-way equality gate, all codec tests, and stuffing fuzz
  tests green. Otherwise revert it and record the result.
- Reuse the main checkout's `external/` fixtures through hard links and treat
  those files as read-only.
- Keep only summary tables and concise analysis under
  `docs/reference/decoders/dng-gap-profiles/`; do not commit raw perf,
  Callgrind, disassembly, or per-run logs.

## Scope

In scope:

- Baseline Fornax and FlashRaw 5D3 counters and assembly inspection.
- One register-resident bit-reader implementation behind the existing row
  decoder.
- One branchless difference-decoder implementation using a compact code table.
- Focused correctness tests for the new reader/token path.
- Final report updates, kept/reverted decisions, dependency-chain comparison,
  branch attribution available on this host, and a scalar-floor conclusion.

Out of scope:

- SIMD, platform-specific code, allocation, map, or thread changes.
- Changing the public decoder API or decoded output.
- Additional LJPEG algorithms beyond the two candidates named above.
- Full corpus tracker runs or RAW payload uploads.
- Merging, closing, or polling the pull request.

## Files Expected to Change

- `crates/fornax-codec/src/ljpeg.rs`
- `docs/reference/decoders/dng-gap-profiles/README.md`
- `docs/reference/decoders/dng-remaining-gap.md` if its LJPEG conclusion changes
- `docs/plans/chore-ljpeg-critical-path.md`

## Task Checklist

- [x] **Baseline:** Confirm the worktree is based on `origin/main`, run codec
  tests, build the probes, and save the 5D3 Fornax/FlashRaw counter table in
  one session.
- [x] **Step 0 assembly:** Capture the final hot-loop disassembly outside git;
  list loop-carried instructions, stack loads/stores, and what keeps each
  value in memory.
- [x] **TDD Experiment 1:** Add a failing behavioral test for invariant reader
  ownership across a whole-row decode if the existing reader tests do not
  cover it. Implement the local-state reader with cold helpers and verify the
  test passes.
- [x] **Measure Experiment 1:** Run all required correctness and performance
  gates. Inspect assembly for reader-state spills. Keep or revert.
- [x] **TDD Experiment 3a:** Add a failing test for branchless difference
  decoding at symbol boundaries, including zero, 15, 16, stuffing, marker
  termination, and a 16-bit Huffman code. Implement the compact code table and
  cold escape path.
- [x] **Measure Experiment 3a:** Run all required correctness and performance
  gates. Keep or revert.
- [x] **Experiment 3b/3c decision:** Continue only if Experiment 3a was kept and
  measured branch misses identify the refill or marker tests as a remaining
  bottleneck. Otherwise record why it is out of scope under the stop rule.
- [x] **Final report:** Record same-session counter tables, wall time, cycles,
  IPC, instructions, branch misses, dependency chains, stack traffic,
  decisions, and the scalar-floor conclusion.

## Validation

Run the focused codec suite:

```bash
cargo test -p fornax-codec --all-features
```

Run the existing stuffing fuzz coverage and build the probes:

```bash
CARGO_PROFILE_RELEASE_DEBUG=line-tables-only \
  cargo test -p fornax-codec --all-features refill
CARGO_PROFILE_RELEASE_DEBUG=line-tables-only \
  cargo build -p fornax-raw-decoder --release --bench dng_gap_probe
CARGO_PROFILE_RELEASE_DEBUG=line-tables-only \
  cargo build -p fornax-codec --release --bench ljpeg_gap_probe
```

Run the exact three-way equality gate for all three real files. Run the
required Fornax/FlashRaw timing matrix in one session and retain only summary
tables. If production code changes, run the repository validation afterwards:

```bash
scripts/agent-validation.sh
```

## Decisions and Notes

- Baseline commit: `985c704` (`perf: close portable lossless JPEG instruction
  gap (#736)`), current `origin/main`.
- The worktree is `.worktrees/chore-ljpeg-critical-path` on branch
  `chore/ljpeg-critical-path`.
- The reader-state candidate is accepted only if it removes the cache stack
  spill and reload from the row loop.
- The branchless-difference candidate is accepted only if its cold escape rate
  and table footprint do not trade lower instruction count for worse cycles or
  wall time.
- If a candidate fails correctness, revert it before reconsidering another.
- Experiment 1 removed the cache stack spill in assembly and reduced 5D3
  instructions/sample to 48.126, but 1T cycles regressed from 23.981 to 24.809,
  so the candidate was reverted in `9a85782`.
- Experiment 3a reduced 5D3 1T cycles/sample from 23.917 to 20.648 and wall time
  from 175.302 ms to 157.404 ms in the same session. The compact 10-bit table
  held only 1,024 samples as cold escapes out of 25,165,824 (0.00407%) and
  reduced L1D load misses from 0.96167 to 0.06010 per sample. The candidate is
  kept in `f06cb2e`.
- Experiment 3b made refill unconditional when eight bytes were available.
  Branch misses/sample fell from 0.10636 to 0.01814 on 5D3, but cycles/sample
  rose from 20.702 to 23.330 and wall time from 162.439 ms to 178.382 ms.
  Every real-file and synthetic case regressed, so the candidate was reverted
  in `0c35bce`.
- No marker-unstuffing change was started. The host cannot sample branch stacks,
  and the branchless refill result showed that removing the remaining refill
  branch does not pay for its extra loads and shifts.
- Validation reached the workspace Clippy stage and stopped on pre-existing
  `fornax-codec` test-support findings (`inline_trait_bounds`, `use_self`, and
  `missing_const_for_fn`) outside this change. The corpus hash, workspace tests,
  docs, codec tests, three-way equality, and lossless-JPEG cross-target checks
  for `x86_64-pc-windows-msvc` and `aarch64-apple-darwin` passed.
- Miri is unavailable because the nightly toolchain has no `cargo-miri`
  component installed. Big-endian Miri therefore remains unrun.
