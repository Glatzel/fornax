# Decoder Golden-Corpus Benchmark Implementation Plan

This plan replaces the single-fixture decoder benchmark with a per-file golden-corpus comparison and a FlashRaw reference.

**Goal:** Benchmark Fornax, Rawler, and FlashRaw on one 2x2 Bayer RAW file per CI matrix item, using only decoder-tracker golden inputs.

**Architecture:** The benchmark parses the pointer-only golden manifest, resolves one or all materialized inputs, and runs a Criterion group per input. Fornax and two benchmark-only adapters expose the shared decoder methods. CI generates one matrix item per golden file and fetches only that file's transient LFS payload.

**Tech Stack:** Rust 2024, Criterion compatibility crate, `flashraw` 0.2.0, `rawler` 0.8.0, decoder-tracker CSV data, GitHub Actions.

## Global Constraints

- Do not modify `TODO.md`, the decoder baseline, or generated README marker regions.
- Do not commit, cache, or upload RAW payloads.
- Keep `rawler` and `flashraw` out of `[workspace.dependencies]`.
- Put `rawler = "0.8.0"` and `flashraw = "=0.2.0"` directly in
  `fornax-decoder` dev-dependencies.
- Existing Rawler test oracles in `fornax-arw`, `fornax-raf`, `fornax-iiq`, and
  `fornax-hasselblad` use direct `rawler = "0.8.0"` dev-dependencies because
  removing the workspace entry would otherwise break those crates.
- Compare only `decode`, `raw_mosaic`, and `cropped_mosaic` across all decoders.
- Keep Fornax `dng_metadata` and `process_metadata` as Fornax-only diagnostics.
- Pin decoder benchmark execution to one thread.
- Do not execute timed benchmarks in the implementation session; GitHub Actions
  owns benchmark execution.

## Review Focus

- A selected benchmark path can bypass golden-manifest membership.
- A reference failure is silently skipped instead of failing its matrix case.
- FlashRaw defaults accidentally apply opcodes, use multiple threads, use the GPU, or change storage-native sample types.
- Removing the Rawler workspace dependency breaks another dev-only test consumer.
- CI fetches, caches, uploads, or iterates more than one RAW payload per matrix item.

## Task 1: Record the plan and establish the ledger

**Files:**

- Create: `docs/plans/chore-decoder-benchmark-golden-corpus.md`

**Interfaces:**

- Consumes: the user-approved benchmark design and dependency ruling.
- Produces: a committed implementation plan and clean baseline.

- [ ] **Step 1: Run focused baseline tests**

Run:

```bash
cargo test -p fornax-decoder -p decoder-tracker --lib
```

Expected: all tests pass.

- [ ] **Step 2: Initialize the per-plan SDD workspace and ledger**

Use `sdd-workspace` with this plan file and create `progress.md` whose first
line names this plan.

- [ ] **Step 3: Commit the plan**

```bash
git add docs/plans/chore-decoder-benchmark-golden-corpus.md
git commit -m "docs: plan decoder golden corpus benchmark"
```

## Task 2: Add benchmark input-selection tests

**Files:**

- Create: `crates/fornax-decoder/tests/benchmark_inputs.rs`

**Interfaces:**

- Consumes: golden CSV bytes and an optional explicit input path.
- Produces: tests for `parse_golden_manifest`, `select_golden_inputs`, and `ensure_materialized` in `benches/decoder_inputs.rs`.

- [ ] **Step 1: Write the failing integration tests**

Include `benches/decoder_inputs.rs` from the integration test with a `#[path]`
module. Cover:

- canonical header parsing and format/label derivation;
- duplicate path rejection;
- invalid SHA-256 rejection;
- selecting all rows when no explicit path is provided;
- selecting exactly one member path;
- rejecting a path outside the golden manifest;
- rejecting an LFS pointer file.

- [ ] **Step 2: Run the tests and confirm RED**

```bash
cargo test -p fornax-decoder --test benchmark_inputs
```

Expected: compilation fails because `benches/decoder_inputs.rs` does not exist.

## Task 3: Implement golden benchmark input selection

**Files:**

- Create: `crates/fornax-decoder/benches/decoder_inputs.rs`
- Modify: `crates/fornax-decoder/Cargo.toml`

**Interfaces:**

- Produces `GoldenInput { path, size, sha256 }`.
- Produces `parse_golden_manifest(&[u8]) -> Result<Vec<GoldenInput>, String>`.
- Produces `select_golden_inputs(&[GoldenInput], Option<&Path>, &Path) -> Result<Vec<(GoldenInput, PathBuf)>, String>`.
- Produces `ensure_materialized(&[(GoldenInput, PathBuf)]) -> Result<(), String>`.

- [ ] **Step 1: Add `csv.workspace = true` to benchmark dev-dependencies**

- [ ] **Step 2: Implement the smallest parser and selector that passes the tests**

Require the exact `path,size,sha256` header, unique normalized repository
paths, 64 hexadecimal SHA-256 characters, and non-pointer materialized files.
Resolve explicit inputs by canonical path membership.

- [ ] **Step 3: Run the integration test and confirm GREEN**

```bash
cargo test -p fornax-decoder --test benchmark_inputs
```

Expected: all benchmark input-selection tests pass.

## Task 4: Add the FlashRaw adapter through TDD

**Files:**

- Create: `crates/fornax-decoder/tests/benchmark_flashraw.rs`
- Create: `crates/fornax-decoder/benches/flashraw_wrapper.rs`
- Modify: `crates/fornax-decoder/Cargo.toml`
- Modify: `Cargo.toml`
- Modify: `crates/fornax-arw/Cargo.toml`
- Modify: `crates/fornax-raf/Cargo.toml`
- Modify: `crates/fornax-iiq/Cargo.toml`
- Modify: `crates/fornax-hasselblad/Cargo.toml`

**Interfaces:**

- Consumes: FlashRaw `RawPixelData`, CFA metadata, and active area.
- Produces: a benchmark-only `FlashRawDecoder` implementing `IDecoder`.
- Produces: `FlashRawDecoder::from_parts(...)` for adapter tests.

**Dependency ruling:** `rawler` and `flashraw` are not workspace dependencies.
`fornax-decoder` owns direct benchmark dev-dependencies. The four existing
Rawler test-oracle crates receive direct dev-dependency pins so the workspace
continues to compile. Cost if wrong: Rawler test-oracle pins are duplicated in
five manifests instead of centralized.

- [ ] **Step 1: Write adapter tests that fail to compile**

Cover:

- preserving `RawPixelData::U16` as `RawMosaic::U16`;
- preserving `RawPixelData::F32` as `RawMosaic::F32`;
- cropping to FlashRaw's active area;
- rejecting non-one-sample data and non-Bayer/no-CFA frames.

- [ ] **Step 2: Confirm RED**

```bash
cargo test -p fornax-decoder --test benchmark_flashraw
```

Expected: compilation fails because the adapter module does not exist.

- [ ] **Step 3: Update direct dev-dependencies**

Remove `rawler` from workspace dependencies. Add direct `rawler` pins where
tests already consume it. Add direct `rawler` and `flashraw` pins to
`fornax-decoder` dev-dependencies.

- [ ] **Step 4: Implement the adapter**

Open with `FlashRaw::open_path`, select `RawF32` or `RawU16` from
`FrameInfo::sample_format`, and decode with:

- CPU backend;
- one thread;
- strict corruption policy;
- opcode application disabled;
- default crop disabled.

Map errors to `DecodeError::Backend` or `Unsupported`, validate dimensions,
sample count, CFA, and active area, clone the storage-native mosaic for
`raw_mosaic`, and crop/convert for `cropped_mosaic`. Return unsupported for
`process_metadata`; do not implement DNG metadata.

- [ ] **Step 5: Confirm GREEN**

```bash
cargo test -p fornax-decoder --test benchmark_flashraw
```

Expected: all adapter tests pass.

## Task 5: Extend and regenerate the golden corpus

**Files:**

- Modify: `crates/decoder-tracker/golden/selected-manifest.csv`
- Regenerate: `crates/decoder-tracker/golden/decode-hashes.csv`

**Interfaces:**

- Consumes: already-passing baseline rows for two 3FR compression families.
- Produces: 30 golden files across eight native formats.

- [ ] **Step 1: Add the two manifest rows**

Add:

```text
Hasselblad/H3D/A9391120.3FR,53594112,34fdb93cb215e4f419ae0f7252c57b881fa381003124b4f9c143d27741f211b3
Hasselblad/X2D 100C/B0000079.3FR,213311488,dcc5a4abe3498e6f25e89bb491995fd12c3b669d9277c71b45b49210d3e56280
```

Keep rows sorted by SHA-256.

- [ ] **Step 2: Initialize the pointer-only corpus**

```bash
GIT_LFS_SKIP_SMUDGE=1 git -c filter.lfs.required=false \
  submodule update --init external/raw.pixls.us
```

- [ ] **Step 3: Regenerate golden hashes**

```bash
cargo run --release -p decoder-tracker -- hash \
  --manifest crates/decoder-tracker/golden/selected-manifest.csv \
  --out crates/decoder-tracker/golden/decode-hashes.csv
```

Expected: generation succeeds for all 30 cases and writes the complete file.

- [ ] **Step 4: Verify golden hashes**

```bash
cargo run --release -p decoder-tracker -- hash \
  --manifest crates/decoder-tracker/golden/selected-manifest.csv \
  --check crates/decoder-tracker/golden/decode-hashes.csv
```

Expected: unchanged.

## Task 6: Rewrite the Criterion benchmark

**Files:**

- Modify: `crates/fornax-decoder/benches/decoder.rs`

**Interfaces:**

- Consumes: `decoder_inputs` and both reference adapters.
- Produces: one Criterion group per golden input and decoder/method cases.

- [ ] **Step 1: Replace fixture resolution**

Read `FORNAX_BENCH_MANIFEST` and `FORNAX_BENCH_CORPUS`, select all rows by
default or one row through `FORNAX_BENCH_RAW`, and reject missing, unmaterialized,
or non-member inputs before benchmark registration.

- [ ] **Step 2: Add FlashRaw to decoder selection**

Accept `fornax`, `rawler`, and `flashraw`; default to all three. Preserve
`FORNAX_BENCH_BITS`.

- [ ] **Step 3: Register grouped measurements**

For every input:

- create one Criterion group named from format and path;
- benchmark Fornax metadata diagnostics once;
- benchmark `decode`, `raw_mosaic`, and `cropped_mosaic` for every selected
  decoder and sample type;
- include the full repository path in startup output;
- fail with input and decoder context instead of an opaque unwrap.

- [ ] **Step 4: Compile**

```bash
cargo bench -p fornax-decoder --bench decoder --no-run
```

Expected: success.

## Task 7: Add the CI per-file matrix

**Files:**

- Create: `crates/decoder-tracker/scripts/test_benchmark_matrix.py`
- Create: `crates/decoder-tracker/scripts/benchmark_matrix.py`
- Modify: `.github/workflows/ci.yml`

**Interfaces:**

- Consumes: `golden/selected-manifest.csv`.
- Produces: GitHub Actions JSON matrix with one unique `path`, `format`, `label`,
  `size`, and `sha256` per item.

- [ ] **Step 1: Write a failing Python unit test**

Test canonical CSV parsing, duplicate path rejection, and one matrix item per
row with populated derived fields.

- [ ] **Step 2: Confirm RED**

```bash
python3 -m unittest crates/decoder-tracker/scripts/test_benchmark_matrix.py
```

Expected: import failure because `benchmark_matrix.py` does not exist.

- [ ] **Step 3: Implement the matrix script and confirm GREEN**

```bash
python3 -m unittest crates/decoder-tracker/scripts/test_benchmark_matrix.py
```

Expected: all tests pass.

- [ ] **Step 4: Update CI**

- Add `decoder-benchmark-plan` and `decoder-benchmark` jobs.
- Generate the dynamic matrix from the golden CSV.
- Initialize the pointer-only corpus at the recorded submodule commit.
- Fetch exactly one matrix path with Git LFS.
- Verify the generated file's SHA-256 against the matrix item.
- Run one input with all three decoders and one thread.
- Do not cache or upload RAW payloads.
- Broaden decoder benchmark change detection and remove `fornax-decoder`
  from the static benchmark matrix.
- Include both new decoder jobs in `check-point`.

## Task 8: Update documentation and dependency references

**Files:**

- Modify: `docs/decoder/benchmark.md`
- Modify: `docs/decoder/corpus.md`
- Modify: `crates/fornax-decoder/README.md`
- Create: `docs/reference/decoders/flashraw.md`
- Modify: `docs/README.md`

**Interfaces:**

- Consumes: final benchmark contract and CI behavior.
- Produces: user-facing and pinned-reference documentation.

- [ ] **Step 1: Rewrite benchmark guidance**

Document golden input resolution, local materialization, per-file CI matrix,
single-thread controls, shared method limitations, and the removal of the old
default fixture.

- [ ] **Step 2: Update golden corpus guidance**

Describe 30 cases across eight native formats, including both 3FR compression
families.

- [ ] **Step 3: Add the FlashRaw reference note**

Pin `flashraw` 0.2.0, record crates.io/GitHub sources, benchmark-only scope,
API differences, required neutral decode options, and MIT OR Apache-2.0
license. Update the documentation index.

- [ ] **Step 4: Run docs validation**

```bash
scripts/agent-validation.sh --docs-only
```

Expected: exit 0.

## Task 9: Validate and review

**Files:**

- Modify: this plan's checklists and decisions as implementation progresses.

**Interfaces:**

- Consumes: all implementation changes.
- Produces: verified branch and pull request.

- [ ] **Step 1: Run focused and dependency checks**

```bash
cargo test -p fornax-decoder --test benchmark_inputs --test benchmark_flashraw
cargo bench -p fornax-decoder --bench decoder --no-run
cargo tree -p fornax-decoder --edges normal
git diff --check
```

Expected: tests compile and pass, the benchmark compiles, and the normal
dependency tree contains neither `rawler` nor `flashraw`.

- [ ] **Step 2: Verify the benchmark matrix in CI**

After pushing the branch, dispatch CI and require the decoder benchmark matrix
to execute one golden input per job with:

```text
RAYON_NUM_THREADS=1
FORNAX_BENCH_RAW=<absolute golden path>
FORNAX_BENCH_DECODER=fornax,rawler,flashraw
FORNAX_BENCH_BITS=32
```

Expected: all three decoders complete the three shared measurements.

- [ ] **Step 3: Run repository validation**

```bash
scripts/agent-validation.sh
```

Expected: pass. Record the complete golden hash check and any fixture or
network limitations.

- [ ] **Step 4: Perform the whole-branch review**

Review the complete diff against `origin/main`, sort findings, fix Critical and
Important findings through RED-GREEN tests, and record minor findings for the
handoff.

- [ ] **Step 5: Commit, push, and open one pull request**

Use a semantic title such as `perf: benchmark decoders across golden corpus`.
Report the PR URL and validation evidence. Do not watch, merge, or close it.

## Decisions and Notes

- Tasks 1-8 and Task 9 steps 1, 3, 4, and 5 are complete. Task 9 step 2 remains
  for the CI run on pull request
  [#717](https://github.com/Glatzel/fornax/pull/717).
- The golden set adds one compression-1 H3D file and one predictor-8 X2D file.
- Rawler and FlashRaw are benchmark development references, never production
  dependencies.
- Rawler remains a dev-only oracle in four format crates, so direct version pins
  replace the removed workspace dependency there.
- FlashRaw is pinned exactly because timing comparisons must not silently move
  to a new decoder release.
- Timing values are review evidence only; no host-dependent baseline is
  committed.
- Actual benchmark timing runs happen only in CI, as requested by the user.
- Self-review found and fixed two benchmark-path defects: explicit single-file
  selection no longer canonicalizes unmaterialized peer files, and an empty
  `FORNAX_BENCH_DECODER` now selects all three decoders as documented.
- FlashRaw `decode` moves the decoded frame data into the adapter instead of
  cloning the full sensor buffer, keeping the timed path free of an extra copy.
- CI review found CodSpeed running the benchmark outside the repository root;
  benchmark paths now resolve relative overrides from `CARGO_MANIFEST_DIR`'s
  repository root instead of the process working directory.
