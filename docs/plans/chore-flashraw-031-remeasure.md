# FlashRaw 0.3.1 Re-measurement Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reorganize decoder reference documentation, then measure FlashRaw 0.3.1 against the pinned 0.2.0 on a reproducible lossless-only DNG/ARW/CR3/NEF matrix and document actionable findings.

**Architecture:** Keep repository work in one isolated branch with separate documentation-reorganization and dependency-update commits. Benchmark the unchanged 0.2.0 tree first, update only the benchmark-only FlashRaw dependency, verify raw-output equivalence, re-run the same inputs, and write the dated evidence under the new `docs/reference/decoders/perf/` hierarchy.

**Tech Stack:** Rust workspace, Cargo/Criterion benchmarks, Pixi environments, decoder-tracker corpus tooling, Git LFS, Markdown documentation.

**Spec:** User request in this session, plus `AGENTS.md`, `.agents/skills/decoder-corpus-tracker/SKILL.md`, and `docs/decoder/benchmark.md`.

## Global Constraints

- Process documentation reorganization before upgrading FlashRaw.
- Benchmark only DNG, Sony ARW, Canon CR3, and Nikon NEF.
- Include only uncompressed and lossless compression modes.
- Exclude Sony lossy/compressed ARW, Canon C-RAW, Nikon lossy and High Efficiency/TicoRAW, lossy DNG, and every other lossy mode from benchmarking, optimization, and reporting.
- Preserve history with `git mv`.
- Do not change decoder behavior or public APIs as part of the documentation move.
- Keep documentation reorganization and the dependency bump in separate commits.
- Do not implement optimization proposals.
- Record sample identity, SHA-256, host, OS, CPU, thread count, Rust toolchain, and profile.
- Report repeated-run variance, decode-only time, and decode -> crop/cast -> preprocess end-to-end time.
- Verify decoded output equivalence before trusting post-upgrade timing.

## Review Focus

- A sample categorized as lossless may actually use a lossy or ambiguous mode; verify from tracker metadata and format evidence before timing.
- Relative links can remain syntactically valid while resolving to the wrong file after a one-level move; validate every local Markdown target.
- FlashRaw `decode`, `raw_mosaic`, crop/cast, and preprocessing can be accidentally conflated; keep the stage split explicit.
- Benchmark noise can masquerade as an upgrade effect; repeat runs and report spread or confidence intervals.
- API or option defaults in FlashRaw 0.3.1 may change thread behavior or output semantics; diff the adapter and release notes before accepting timings.

---

### Task 1: Reorganize decoder reference documentation

**Files:**
- Move: `docs/reference/decoders/*.md` into `perf/`, `design/`, or `reference/`
- Move: `docs/reference/decoders/arw-gap-profiles/` to `docs/reference/decoders/perf/arw-gap-profiles/`
- Move: `docs/reference/decoders/dng-gap-profiles/` to `docs/reference/decoders/perf/dng-gap-profiles/`
- Create: `docs/reference/decoders/README.md`
- Modify: every tracked file containing an old decoder-reference path or affected relative link

**Interfaces:**
- Consumes: the confirmed old-to-new mapping posted before the move.
- Produces: `docs/reference/decoders/{perf,design,reference}/` and a root decoder-reference index.

- [x] **Step 1: Move each mapped file or evidence directory with `git mv`**
- [x] **Step 2: Rewrite repository-wide old paths to their new category-qualified paths**
- [x] **Step 3: Validate all local Markdown link targets and search for every old filename/path**
- [x] **Step 4: Add the decoder-reference README with folder definitions and complete contents**
- [x] **Step 5: Run the available docs build or link checker and commit the documentation change**

### Task 2: Build and verify the lossless-only benchmark matrix

**Files:**
- Inspect: `crates/decoder-tracker/data/manifest.csv`
- Inspect: `crates/decoder-tracker/data/baseline.csv`
- Inspect: `crates/decoder-tracker/golden/selected-manifest.csv`
- Create: transient benchmark matrix under `target/flashraw-031/`

**Interfaces:**
- Consumes: tracker paths, sizes, SHA-256 values, format, compression, resolution, and bit-depth metadata.
- Produces: a reviewed sample list used unchanged for both FlashRaw versions, plus explicit exclusions and coverage gaps.

- [ ] **Step 1: Join the manifest and baseline to enumerate DNG/ARW/CR3/NEF samples**
- [ ] **Step 2: Classify each candidate as uncompressed, lossless, or lossy and record dimensions and bit depth**
- [ ] **Step 3: Remove out-of-scope lossy rows and record each exclusion**
- [ ] **Step 4: Materialize a representative matrix with at least one uncompressed and one lossless sample per format where available**
- [ ] **Step 5: Record checksums, file sizes, camera metadata, and explicit coverage gaps**

### Task 3: Establish FlashRaw 0.2.0 baseline

**Files:**
- Inspect: `crates/fornax-raw-decoder/Cargo.toml`
- Inspect: `crates/fornax-raw-decoder/benches/decoder.rs`
- Inspect: `crates/fornax-raw-decoder/benches/flashraw_wrapper.rs`
- Create: transient raw measurements under `target/flashraw-031/`

**Interfaces:**
- Consumes: the frozen Task 2 matrix and current `flashraw = "=0.2.0"` dependency.
- Produces: repeated pre-upgrade measurements and output hashes.

- [ ] **Step 1: Record host, toolchain, profile, thread configuration, and sample hashes**
- [ ] **Step 2: Verify every matrix file decodes successfully in both Fornax and FlashRaw**
- [ ] **Step 3: Measure repeated FlashRaw decode and the Fornax pipeline stages over the identical matrix**
- [ ] **Step 4: Save raw-output hashes for bit-for-bit comparison**

### Task 4: Upgrade to FlashRaw 0.3.1 and re-measure

**Files:**
- Modify: `crates/fornax-raw-decoder/Cargo.toml`
- Modify: `Cargo.lock`
- Modify as required: FlashRaw adapter and tests
- Inspect upstream: FlashRaw 0.2.0 and 0.3.1 release/source differences

**Interfaces:**
- Consumes: Task 3 baseline and matrix.
- Produces: upgraded dependency, output-equivalence evidence, and post-upgrade repeated measurements.

- [ ] **Step 1: Read release notes and diff the dependency range for API, behavior, and performance changes**
- [ ] **Step 2: Update the exact pin to 0.3.1 and resolve build breakage**
- [ ] **Step 3: Run focused adapter and equivalence tests**
- [ ] **Step 4: Re-run the unchanged matrix and verify baseline output hashes**
- [ ] **Step 5: Compute per-sample deltas and variance, separating decode and end-to-end time**
- [ ] **Step 6: Commit the dependency bump separately**

### Task 5: Investigate remaining opportunities and publish evidence

**Files:**
- Create: `docs/reference/decoders/perf/flashraw-0.3.1-remeasure-2026-11-16.md`
- Modify: `docs/reference/decoders/README.md`
- Modify: `docs/README.md`
- Modify as required: related decoder reference notes

**Interfaces:**
- Consumes: Task 2 matrix, Task 3 baseline, Task 4 measurements and upstream diff.
- Produces: reproducible methodology, sample matrix, deltas, path-stage profiling, and ranked proposals.

- [ ] **Step 1: Profile end-to-end stage costs and compare uncompressed against lossless per brand**
- [ ] **Step 2: Separate FlashRaw cost from adapter, crop/cast, preprocess, allocation, and memory effects**
- [ ] **Step 3: Identify fastest applicable FlashRaw API/options and upstream-worthy issues**
- [ ] **Step 4: Write the dated performance note with all methodology and raw evidence paths**
- [ ] **Step 5: Run documentation/link validation and repository-required validation**

## Validation

```bash
scripts/agent-validation.sh --docs-only
cargo test -p fornax-raw-decoder
cargo test -p fornax-raw-decoder --test benchmark_flashraw
cargo test -p fornax-raw-decoder --test decoder_equivalence -- --ignored --nocapture
cargo bench -p fornax-raw-decoder --bench decoder --no-run
git diff --check
```

Run workspace `scripts/agent-validation.sh` after the dependency change unless the repository
revision removes that script; in that case run the equivalent current validation commands
listed by the repository. Benchmark commands and results are recorded in the performance note.

## Decisions and notes

- The docs tree will use `perf/`, `design/`, and `reference/` under
  `docs/reference/decoders/`.
- `flashraw.md` remains intact under `reference/` because its dominant role is the
  adapter/API contract; its benchmark notes are documented as mixed content.
- `dng-remaining-gap.md` remains intact under `perf/` because its dominant role is
  performance evidence; its allocator design decisions are documented as mixed content.
- FlashRaw remains benchmark-only and must not enter production decoder dependencies.
- RAW payloads remain transient and are never committed.
