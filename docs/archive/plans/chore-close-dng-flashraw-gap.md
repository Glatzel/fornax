> Status: archived completed plan.
> Merged: #724 `chore/close-dng-flashraw-gap` on 2026-11-15.
> Current guidance: [current documentation](../reference/decoders/perf/dng-remaining-gap.md).

# DNG FlashRaw Gap Measurement and Targeted Closure Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> `superpowers:executing-plans` to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Measure the remaining Fornax-to-FlashRaw DNG decode gap, retain only
optimizations that exceed process noise, and replace the unmeasured statements
in the DNG gap report with evidence.

**Architecture:** A harness-free probe binary reuses the existing benchmark
adapters and exact three-way mosaic equality gate, but accepts the four report
inputs directly because two 5D3 files are no longer in the selected manifest.
Perf-stat data and DWARF-call-graph flamegraphs establish where the gap remains
before the uncompressed allocation and LJPEG hypotheses are tested
independently. Production changes remain conditional on repeated before/after
measurements.

**Tech Stack:** Rust 2024, release builds with DWARF debug info, `perf`,
Rayon limited to one worker, existing Fornax/Rawler/FlashRaw adapters, and
Criterion-compatible workspace dependencies. A benchmark-only `libc` dependency
is allowed only for the requested `alloc_zeroed` plus `MADV_HUGEPAGE`
experiment.

**Spec:** The 2026-11-15 task request reproduced by this plan and the current
[DNG remaining performance gap report](../reference/decoders/perf/dng-remaining-gap.md).

## Global Constraints

- Use one thread, release optimization, a page-cache-hot input, and the same
  host settings as the report.
- Run dimensions, storage type, sample bits, and exact pixels through the
  three-way equality gate before trusting a timing.
- Test one hypothesis per production change.
- Benchmark before and after with enough runs to estimate process noise.
- Revert every change whose effect is within noise.
- Prefer safe Rust. A measured gain must justify any retained unsafe code.
- Do not optimize Deflate in this task; profile it only if steps 1 through 3
  finish.
- Stop after updating `docs/reference/decoders/perf/dng-remaining-gap.md`.

## Scope

In scope:

- 5D3 uncompressed, 5D3 LJPEG, Pixel 8 LJPEG, and 5D3 HDR FP32 Deflate.
- Instruction, branch-miss, and minor-fault counters plus DWARF flamegraphs.
- Uncompressed fresh, reused, and zeroed-hugepage destination experiments.
- Synthetic LJPEG scan generation for one/two components, predictor 1,
  smooth/noisy data, and configurable tile counts.
- Independent LJPEG experiments for unstuffing, component specialization,
  buffered errors, and Huffman-table reuse, followed by a combined-winner run.
- Exact evidence-based updates to the remaining-gap report.

Out of scope:

- Deflate implementation changes.
- SIMD, target-CPU, compiler-flag, or backend swaps.
- Multi-thread scaling and cold-storage measurements.
- Changes to raw semantics, crop behavior, metadata normalization, or output
  conversion.

## Review Focus

- An LJPEG fast path must remain bit-identical to the general path for one and
  two components, smooth and noisy entropy data, marker stuffing, truncated
  data, and multiple tiles.
- Huffman table reuse must key on complete table definitions and never reuse a
  table across differing definitions or malformed streams.
- Allocation-policy measurements must separate allocation/fault cost from TIFF
  assembly and sample copy rather than attributing all end-to-end time to one
  stage.
- Perf ratios must compare counters from the same host, build profile,
  workload, thread count, and process run count.
- Report conclusions must distinguish profile share from gap contribution.

## Files Expected To Change

- Create: `crates/fornax-raw/benches/dng_gap_probe.rs`
- Modify: `crates/fornax-raw-decoder/Cargo.toml`
- Create: `crates/fornax-codec/benches/ljpeg_gap_probe.rs`
- Modify: `crates/fornax-codec/Cargo.toml`
- Modify, only after measurement: `crates/fornax-codec/src/uncompressed.rs`
- Modify, only after measurement: `crates/fornax-codec/src/samples.rs`
- Modify, only after measurement: `crates/fornax-codec/src/ljpeg.rs`
- Create: `docs/reference/decoders/perf/dng-gap-profiles/*.svg`
- Create: `docs/reference/decoders/perf/dng-gap-profiles/perf-stat.txt`
- Create: `docs/reference/decoders/perf/dng-gap-profiles/timings.txt`
- Modify: `docs/reference/decoders/perf/dng-remaining-gap.md`
- Modify: `docs/plans/chore-close-dng-flashraw-gap.md`

## Task Checklist

- [x] Task 1: Add the exact-input probe and reproducibility helpers.
- [x] Task 2: Capture the four-case counters and DWARF flamegraphs.
- [x] Task 3: Measure uncompressed allocation and first-touch policies.
- [x] Task 4: Generate deterministic 1/2-component LJPEG scan fixtures.
- [x] Task 5: Test hypotheses A through D independently.
- [x] Task 6: Benchmark combined winners and revert noise-level changes.
- [x] Task 7: Replace unmeasured report statements and stop.

---

## Task 1: Exact-input probe and reproducibility helpers

**Files:**

- Create: `crates/fornax-raw/benches/dng_gap_probe.rs`
- Modify: `crates/fornax-raw-decoder/Cargo.toml`

**Interfaces:**

- Produces:

```rust
enum ProbeDecoder {
    Fornax,
    FlashRaw,
}

enum ProbeMode {
    Decode,
    Equality,
}

struct ProbeArgs {
    mode: ProbeMode,
    decoder: ProbeDecoder,
    rounds: usize,
    warmups: usize,
    input: PathBuf,
}
```

- [x] **Step 1: Add the failing bench target**

Declare a harness-free `dng_gap_probe` benchmark in
`crates/fornax-raw-decoder/Cargo.toml`, then run:

```bash
cargo bench -p fornax-raw-decoder --bench dng_gap_probe -- --help
```

Expected: FAIL because the benchmark source does not exist.

- [x] **Step 2: Implement argument parsing and the equality-only mode**

Reuse `flashraw_wrapper`, `rawler_wrapper`, and `mosaic_compare`. `equality`
must decode all three decoders and require exact dimensions, storage type,
sample bits, and pixels. Print one `PASS <decoder> <width>x<height> <storage>`
line per decoder.

- [x] **Step 3: Implement isolated decode mode**

`decode --decoder fornax|flashraw --warmups N --rounds N PATH` must run the
headline operation used by the report: Fornax `decode(path).raw_mosaic()` and
FlashRaw `decode(path)`. Reject a `RAYON_NUM_THREADS` value other than `1`.

- [x] **Step 4: Verify the probe**

Run:

```bash
RAYON_NUM_THREADS=1 cargo bench -p fornax-raw-decoder --bench dng_gap_probe -- equality 'external/raw.pixls.us/Adobe DNG Converter/Canon EOS 5D Mark III/5G4A9394-uncompressed.DNG'
RAYON_NUM_THREADS=1 cargo bench -p fornax-raw-decoder --bench dng_gap_probe -- decode --decoder fornax --warmups 1 --rounds 1 'external/raw.pixls.us/Adobe DNG Converter/Canon EOS 5D Mark III/5G4A9394-uncompressed.DNG'
```

Expected: PASS, then one finite decode timing.

- [x] **Step 5: Record the exact input paths in the plan decisions**

Use:

- 5D3 uncompressed:
  `Adobe DNG Converter/Canon EOS 5D Mark III/5G4A9394-uncompressed.DNG`
- 5D3 LJPEG:
  `Adobe DNG Converter/Canon EOS 5D Mark III/5G4A9394-compressed-lossless.DNG`
- Pixel 8 LJPEG:
  `Google/Pixel 8 Pro/PXL_20240415_103400204.RAW-02.ORIGINAL.dng`
- 5D3 HDR FP32 Deflate:
  `Canon/EOS 5D Mark III/hdrmerge-bayer-fp32-w-pred-deflate.dng`

All paths are relative to the main checkout's `external/raw.pixls.us` because
the isolated worktree's raw.pixls.us submodule is pointer-only.

## Task 2: Capture counters and DWARF flamegraphs

**Files:**

- Create: `docs/reference/decoders/perf/dng-gap-profiles/perf-stat.txt`
- Create: `docs/reference/decoders/perf/dng-gap-profiles/timings.txt`
- Create: `docs/reference/decoders/perf/dng-gap-profiles/{case}-{decoder}.svg`

**Interfaces:**

- Consumes: Task 1 probe.
- Produces: one counter row and one flamegraph per decoder for each of the four
  inputs, plus interleaved timing distributions.

- [x] **Step 1: Build the probe once with release optimization and DWARF**

Run:

```bash
CARGO_PROFILE_RELEASE_DEBUG=2 CARGO_PROFILE_RELEASE_STRIP=none \
  cargo build -p fornax-raw-decoder --release --bench dng_gap_probe
```

Expected: release probe binary with nonempty debug information.

- [x] **Step 2: Warm the page cache**

Read each input once in full, then run one untimed Fornax decode.

- [x] **Step 3: Capture counters**

For each input and decoder, run the probe under:

```bash
perf stat -e instructions,branch-misses,minor-faults \
  -- ./target/release/deps/dng_gap_probe-* decode \
  --decoder <fornax|flashraw> --warmups 1 --rounds <N> <input>
```

Use `N=5` for 5D3 uncompressed and LJPEG and Deflate; use `N=3` for Pixel 8
LJPEG if profile duration makes 5 excessive. Record raw output and derive
per-decode counters.

- [x] **Step 4: Capture DWARF call graphs**

For each input and decoder, run:

```bash
perf record --call-graph dwarf,16384 -F 999 -o <case>-<decoder>.data \
  -- ./target/release/deps/dng_gap_probe-* decode \
  --decoder <fornax|flashraw> --warmups 1 --rounds <N> <input>
perf script -i <case>-<decoder>.data > <case>-<decoder>.perf-script
```

Generate folded stacks and an SVG from the DWARF call stacks, save the SVG under
`docs/reference/decoders/perf/dng-gap-profiles/`, and remove raw `perf.data` files.

- [x] **Step 5: Capture interleaved timings**

Run at least three fresh processes per case and decoder with interleaving at the
worker level. Save means, standard deviations, minima, medians, and maxima to
`timings.txt`.

## Task 3: Uncompressed allocation and first-touch experiment

**Files:**

- Create: `crates/fornax-codec/benches/ljpeg_gap_probe.rs` initially for the
  shared measurement harness
- Modify: `crates/fornax-codec/Cargo.toml`
- Modify only if selected: `crates/fornax-codec/src/uncompressed.rs`
- Modify only if selected: `crates/fornax-codec/src/samples.rs`

**Interfaces:**

- Consumes: the in-memory 5D3 uncompressed U16 payload and its `DecodeRequest`.
- Produces: timing for:

```rust
fresh_vec_decode()
reused_pretouched_vec_decode()
alloc_zeroed_madv_hugepage_decode()
```

- [x] **Step 1: Add the failing uncompressed benchmark**

Declare a harness-free codec benchmark and implement a mode that asserts the
three variants produce identical U16 dimensions and bits. Run it before all
three variants exist.

Expected: FAIL on the first missing variant.

- [x] **Step 2: Implement the fresh baseline**

Call the existing public uncompressed decode path from an in-memory source and
record total codec time.

- [x] **Step 3: Measure the reused pre-touched destination**

Add a benchmark-only destination kernel that clears or reuses a pre-faulted
output allocation while preserving exact samples. Compare at least five fresh
processes against the fresh baseline.

- [x] **Step 4: Measure `alloc_zeroed` plus `MADV_HUGEPAGE`**

Use `libc::alloc_zeroed` and `libc::madvise(..., MADV_HUGEPAGE)` in the
benchmark only. Keep the unsafe block narrowly documented and prove output
equality before timing.

- [x] **Step 5: Report the four-way time split**

Use Task 2 counter evidence and probe phases to report read/map, TIFF assembly,
allocation/first touch, and sample copy separately. Recommend a production
allocation policy only if reused or hugepage allocation closes most of the
5D3-uncompressed gap.

- [x] **Step 6: Apply or revert**

If a policy wins outside noise, add a failing production test for the selected
behavior, implement the smallest safe change, and rerun the three-way equality
gate plus the benchmark. Otherwise record the measured loss and keep production
code unchanged.

## Task 4: Synthetic LJPEG scan generator

**Files:**

- Modify: `crates/fornax-codec/benches/ljpeg_gap_probe.rs`
- Modify only if a public test hook is required:
  `crates/fornax-codec/src/ljpeg.rs`

**Interfaces:**

- Produces:

```rust
enum ScanPattern {
    Smooth,
    Noisy,
}

struct ScanSpec {
    components: u8,
    pattern: ScanPattern,
    width: u16,
    height: u16,
    tiles: usize,
}

fn synthetic_scan(spec: &ScanSpec) -> Vec<Vec<u8>>;
```

- [x] **Step 1: Write the failing synthetic generator test**

Test one and two components, predictor 1, smooth and noisy samples, exact sample
counts, deterministic bytes, stuffed `0xff` handling, and tile counts 1, 64, and
256.

- [x] **Step 2: Implement the generator**

Generate predictor-1 samples with deterministic pseudo-random state. Smooth data
uses local deltas in `[-1, 1]`; noisy data spans the configured bit depth.
Encode each tile with an interleaved predictor-1 lossless-JPEG stream.

- [x] **Step 3: Verify round trips**

Decode every generated tile with the production fast path and the forced
general path and require exact sample equality.

## Task 5: Test LJPEG hypotheses independently

**Files:**

- Modify: `crates/fornax-codec/benches/ljpeg_gap_probe.rs`
- Modify one hypothesis in `crates/fornax-codec/src/ljpeg.rs` at a time

**Interfaces:**

- Consumes: Task 4 generator and production scan decoding.
- Produces: before/after distributions for A, B, C, and D.

- [x] **Step 1: Benchmark the baseline matrix**

Measure one- and two-component, smooth and noisy scans at 1, 64, and 256 tiles,
plus the three real LJPEG corpus cases. Use at least five fresh processes.

- [x] **Step 2: Test hypothesis A, up-front `0xff00` unstuffing**

Write a failing test that compares stuffed and unstuffed decoding, implement one
copy-once unstuffing path with a plain bit pump, and benchmark independently.
Revert if the result is within noise.

- [x] **Step 3: Test hypothesis B, const-generic component loops**

Write a failing one/two-component equality test, specialize the row loop without
changing table or error semantics, and benchmark independently. Revert if the
result is within noise.

- [x] **Step 4: Test hypothesis C, buffered errors with a cold path**

Write malformed-input tests for invalid code, truncated amplitude, and marker
termination. Buffer the hot-path error flag, branch to the existing cold error
path, and benchmark independently. Revert if the result is within noise.

- [x] **Step 5: Test hypothesis D, Huffman table reuse**

Write tests for identical reuse, one-byte table changes, malformed definitions,
and multi-table streams. Reuse only complete byte-identical parsed tables and
benchmark independently. Revert if the result is within noise.

## Task 6: Combined winners

**Files:**

- Modify only winner paths identified in Task 5.
- Modify benchmark and reference notes as needed.

- [x] **Step 1: Combine only winners**

Apply winners one at a time in descending measured effect, rerunning equality
and the real cases after each.

- [x] **Step 2: Rebenchmark all three real LJPEG inputs**

Run at least five fresh processes per decoder, interleaved with the baseline
binary when possible, and report before/after per file.

- [x] **Step 3: Revert combined regressions**

Revert any winner that loses its gain when combined or pushes another measured
case into a statistically distinguishable regression.

## Task 7: Report and stop

**Files:**

- Modify: `docs/reference/decoders/perf/dng-remaining-gap.md`
- Modify: `docs/plans/chore-close-dng-flashraw-gap.md`

- [x] **Step 1: Replace every remaining “not measured” statement**

Include measured shares, confidence intervals or observed run spread, kept and
reverted hypotheses, and profile links.

- [x] **Step 2: State the allocation recommendation**

Recommend an allocation policy only if the uncompressed experiment closes most
of the 5D3 gap.

- [x] **Step 3: Run focused tests**

Run:

```bash
cargo test -p fornax-codec --features lossless-jpeg,deflate
cargo bench -p fornax-codec --bench ljpeg_gap_probe -- --self-check
cargo bench -p fornax-raw-decoder --bench dng_gap_probe -- equality <each input>
```

Expected: all equality and unit checks pass.

- [x] **Step 4: Run workspace validation**

Run `scripts/agent-validation.sh` after implementation changes. If the script
attempts network or fixture setup blocked by the environment, report the exact
blocked check and run every local task-specific check that remains available.

The full workspace validation was started, completed workspace tests and
doctests, then hit repeated clippy fixes. Per explicit user instruction after
the final focused-test pass, the remaining full validation rerun was skipped.

- [x] **Step 5: Inspect the diff, commit, open the pull request, and stop**

Do not optimize Deflate. Do not add speculative follow-up code. Commit the plan,
measurement artifacts, selected changes, and report update in one semantic pull
request. Pull request:
[#724](https://github.com/Glatzel/fornax/pull/724).

## Validation

- `cargo test -p fornax-codec --features lossless-jpeg,deflate`
- `cargo bench -p fornax-codec --bench ljpeg_gap_probe -- --self-check`
- `cargo bench -p fornax-raw-decoder --bench dng_gap_probe -- equality <input>`
  for all four inputs
- Repeated release/perf measurements recorded under
  `docs/reference/decoders/perf/dng-gap-profiles/`
- `scripts/agent-validation.sh`

## Decisions and Notes

- The isolated worktree uses the main checkout's materialized
  `external/raw.pixls.us`; the submodule is pointer-only here and must not be
  modified.
- Criterion is the workspace benchmark dependency, but the exact-input and perf
  probes use harness-free binaries so profiling runs a known number of decodes.
- Profile shares describe time within one decoder; gap contributions require
  comparing Fornax and FlashRaw wall time or counters for the same workload.
- The existing equality helper printed pixel mismatches without failing. It was
  fixed with a failing regression test before any performance measurement.
- The two Adobe-converted 5D3 files are present in the main checkout but are not
  in the selected manifest. The probe accepts explicit paths and still evaluates
  the full three-way equality gate.
- Hypothesis C is retained. A, B, and D were reverted after independent or
  combined measurements.
- Pentax K-5 is pointer-only in this checkout and Fornax has no Pentax container
  decoder, so the final real-file LJPEG comparison covers 5D3, Pixel 8, and
  iPhone SE.
- Deflate receives counter/flamegraph evidence only. Its implementation remains
  unchanged in this task.
