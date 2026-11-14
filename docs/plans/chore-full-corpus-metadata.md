# Full-Corpus Encoding Metadata and Golden Coverage Audit

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the DNG-only corpus metadata sidecar with normalized
source-encoding metadata in the canonical all-format decoder baseline, then
use that metadata to rebuild the golden corpus with one lowest-pixel
representative per known-lossless encoding group.

**Architecture:** Each native decoder exposes a small, codec-independent
`EncodingMetadata` value backed by its already parsed source layout. The
decoder facade dispatches that value, and `decoder-tracker` writes it into the
same row as the decoder result. A separate read-only command compares the full
baseline with the golden manifest and reports missing or redundant encoding
coverage. Container and codec crates are not given raw-format tracking logic.

**Tech Stack:** Rust workspace, pure Rust decoders, CSV, DuckDB, GitHub Actions,
and Pixi-managed tooling.

**Spec:** User-approved decisions in the `chore/full-corpus-metadata` request:
merge metadata into the CSV baseline, retain all corpus rows, and audit golden
coverage before selecting one lossless file per encoding group.

## Global Constraints

- Start from the merged `origin/main` and work only in
  `.worktrees/chore-full-corpus-metadata`.
- Do not commute RAW payloads. Do not stage `external/raw.pixls.us`.
- Do not hand-edit generated corpus CSVs or the generated README marker region.
- Do not add dependencies.
- Keep container parsing in `fornax-tiff`, codec behavior in `fornax-codec`,
  and DNG/vendor-specific source classifications in their owning format crates.
- Keep all 2,020 corpus keys in the baseline. Formats without a native decoder
  keep the row with `compression_name=unknown`, `storage=unknown`, and empty
  optional fields.
- Generate the golden manifest and hashes from known-lossless encoding groups,
  retaining one representative per group.
- Do not edit `TODO.md`; this is an explicit maintenance request rather than a
  bracketed backlog item.

## Review Focus

- A source metadata label loses a decoder variant distinction, causing two
  different codec paths to look redundant to the golden audit.
- DNG storage metadata is taken from normalized DNG output rather than the
  selected raw IFD, hiding strip/tile and predictor differences.
- The appended baseline columns cause an existing status, hash, dimension, CFA,
  or stage value to change during the full-corpus migration.
- A fully passing row has incomplete metadata that makes golden coverage
  silently undercount a decoder path.
- An unsupported or parse-failed file is mistaken for an infrastructure failure
  when it should remain a normal baseline result.

---

## Task 1: Worktree and plan

**Files:**

- Create: `docs/plans/chore-full-corpus-metadata.md`

**Interfaces:**

- Consumes: merged `origin/main`.
- Produces: a clean branch and a committed implementation plan.

- [x] **Step 1: Create the isolated worktree**

```bash
git worktree add -b chore/full-corpus-metadata \
  .worktrees/chore-full-corpus-metadata origin/main
```

- [x] **Step 2: Verify the baseline**

Run:

```bash
git status --short --branch --ignore-submodules=all
cargo test -p decoder-tracker
```

Expected: the branch matches `origin/main` and the tracker tests pass.

- [ ] **Step 3: Commit the plan**

```bash
git add docs/plans/chore-full-corpus-metadata.md
git commit -m "docs: plan full corpus encoding metadata"
```

---

## Task 2: Add Source Encoding Metadata

**Files:**

- Modify: `crates/fornax-raw-core/src/{lib.rs,metadata.rs,protocol.rs}`
- Modify: `crates/fornax-dng/src/dng_decoder/mod.rs`
- Modify: each `crates/fornax-raw-{canon,fujifilm,hasselblad,nikon,panasonic,phase-one,sony}/src/lib.rs`
- Modify: `crates/fornax-raw-decoder/src/lib.rs`
- Test: focused unit tests in the affected crates

**Interfaces:**

- Consumes: parsed source layouts already retained by every native decoder.
- Produces:

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EncodingSampleFormat {
    Integer,
    Float,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EncodingMetadata {
    pub compression_id: Option<u32>,
    pub compression_name: &'static str,
    pub storage: &'static str,
    pub bits_per_sample: Option<u16>,
    pub sample_format: Option<EncodingSampleFormat>,
    pub predictor: Option<u16>,
    pub rows_per_strip: Option<u32>,
    pub tile_width: Option<u32>,
    pub tile_height: Option<u32>,
}

pub trait IDecoderEncoding {
    fn encoding_metadata(&self) -> EncodingMetadata;
}
```

- [x] **Step 1: Write failing metadata contract tests**

Add tests proving:

- DNG tiled and striped lossless JPEG preserve compression, storage, bit depth,
  sample format, predictor, and tile/strip geometry.
- Sony maps uncompressed, ARW1, ARW2, ARW6, and lossless-JPEG families.
- Nikon distinguishes uncompressed words, packed modes, striped 12-bit, and
  Huffman mode.
- Canon reports CRX, bit depth, BMFF storage, and CRX tile dimensions.
- Fujifilm maps packed-12, words, and lossless modes.
- Hasselblad maps uncompressed and lossless JPEG.
- Panasonic maps V4, V5, V6, V8, unpacked 12-bit, and packed 12-bit control.
- Phase One maps uncompressed, lossless 14, lossless 16, nonlinear, and SV2.

- [x] **Step 2: Run the tests and verify the contract is missing**

```bash
cargo test -p fornax-raw-core
cargo test -p fornax-dng
cargo test -p fornax-raw-sony
cargo test -p fornax-raw-nikon
cargo test -p fornax-raw-canon
cargo test -p fornax-raw-fujifilm
cargo test -p fornax-raw-hasselblad
cargo test -p fornax-raw-panasonic
cargo test -p fornax-raw-phase-one
cargo test -p fornax-raw-decoder
```

Expected: compile failures for the missing metadata type and methods.

- [x] **Step 3: Implement the shared metadata type and per-format mappings**

Use source layout fields rather than normalized DNG metadata for source bit
depth. Use `format` to scope numeric IDs. DNG uses the full-resolution raw IFD;
vendor names are stable and namespaced where an ID alone is ambiguous.

- [x] **Step 4: Re-run the focused tests**

Expected: all focused test targets pass.

- [x] **Step 5: Commit**

```bash
git add crates/fornax-raw-core crates/fornax-dng \
  crates/fornax-raw-canon crates/fornax-raw-fujifilm \
  crates/fornax-raw-hasselblad crates/fornax-raw-nikon \
  crates/fornax-raw-panasonic crates/fornax-raw-phase-one \
  crates/fornax-raw-sony crates/fornax-raw-decoder
git commit -m "feat: expose source encoding metadata"
```

---

## Task 3: Merge Metadata Into the Canonical Baseline

**Files:**

- Modify: `crates/decoder-tracker/src/{model.rs,stages.rs,csvio.rs,duckdb.rs,main.rs}`
- Modify: `crates/decoder-tracker/sql/{merge.sql,diff.sql,upsert.sql}`
- Modify: `crates/decoder-tracker/scripts/aggregate.sh`
- Modify: `.github/workflows/decoder-tracker-corpus.yml`
- Delete: `crates/decoder-tracker/src/corpus_metadata.rs`
- Delete: `crates/decoder-tracker/data/corpus-metadata.csv`
- Test: `crates/decoder-tracker/src/tests.rs`,
  `crates/decoder-tracker/tests/cli.rs`

**Interfaces:**

- Consumes: `Decoder::encoding_metadata()` from Task 2.
- Produces: the canonical baseline header:

```csv
status,camera,format,path,decode_pass,raw_mosaic_pass,raw_sha256,process_metadata_pass,handoff_pass,error_kind,width,height,cfa,compression_id,compression_name,compression_lossless,storage,bits_per_sample,sample_format,predictor,rows_per_strip,tile_width,tile_height
```

- [x] **Step 1: Write failing baseline schema and stage tests**

Test complete metadata for a passing decoder, unknown metadata for an
unsupported file, CSV round trips, canonical ordering, DuckDB merge/diff/upsert,
and rejection of a fully passing row with incomplete source metadata.

- [x] **Step 2: Run the tests and verify the schema mismatch**

```bash
cargo test -p decoder-tracker
pixi run -e tracker cargo test -q -p decoder-tracker duckdb_
```

Expected: failures from the missing fields and old header.

- [x] **Step 3: Implement the schema and workflow changes**

Carry metadata through `StageRow` and `BaselineRow`. Preserve metadata when a
decoder was constructed even if a later stage fails. Update every SQL projection
and shell schema check. Remove the second `corpus-metadata` execution and its
artifact from the workflow.

- [x] **Step 4: Prove DNG parity before deleting the sidecar**

Generate a selected DNG baseline and compare its metadata columns with the
existing `data/corpus-metadata.csv` for all 262 sidecar rows. Compression,
storage, bit depth, sample format, predictor, and geometry must match exactly.

- [x] **Step 5: Re-run tracker tests**

Expected: Rust and DuckDB tests pass.

- [x] **Step 6: Commit**

```bash
git add crates/decoder-tracker .github/workflows/decoder-tracker-corpus.yml
git commit -m "refactor: merge encoding metadata into corpus baseline"
```

---

## Task 4: Add Golden Coverage Audit

**Files:**

- Modify: `crates/decoder-tracker/src/{lib.rs,main.rs,golden.rs}`
- Test: tracker unit and CLI tests
- Modify: `docs/decoder/corpus.md`,
  `.agents/skills/decoder-corpus-tracker/SKILL.md`,
  `crates/decoder-tracker/AGENTS.md`

**Interfaces:**

- Consumes: the all-format metadata baseline and pointer-only manifests.
- Produces:

```text
decoder-tracker golden-coverage \
  --baseline FILE --corpus-manifest FILE --golden FILE [--out FILE]
  [--selected-out FILE] [--exclude PATH]...
```

- [x] **Step 1: Write failing golden coverage tests**

Test groups by the lossless encoding tuple
`(format, compression_name, bits_per_sample, sample_format, storage, predictor)`,
lowest-pixel representative selection, missing golden coverage, duplicate
golden groups, and rejection of absent or non-passing golden keys.

- [x] **Step 2: Run the tests and verify the command is missing**

```bash
cargo test -p decoder-tracker golden_coverage
```

Expected: compile or assertion failure because the command does not exist.

- [x] **Step 3: Implement the deterministic CSV audit and summary**

Write `target/decoder-tracker/golden-coverage.csv` by default. Emit passing
counts, golden counts, golden paths, and the lowest-pixel passing
representative. `--selected-out` emits one representative per
`compression_lossless=1` encoding group. `--exclude` skips passing candidates
whose LFS objects are unavailable.

- [x] **Step 4: Re-run tracker tests and inspect a generated report**

Expected: tests pass and the report is sorted deterministically.

- [x] **Step 5: Commit**

```bash
git add crates/decoder-tracker docs/decoder/corpus.md .agents/skills
git commit -m "feat: audit golden corpus encoding coverage"
```

---

## Task 5: Full-Corpus Migration and Acceptance

**Files:**

- Generated: `crates/decoder-tracker/data/baseline.csv`
- Generated: the README decoder-tracker marker region

**Interfaces:**

- Consumes: the existing full-corpus workflow.
- Produces: a reviewed, complete 2,020-row baseline containing source metadata.

- [x] **Step 1: Run repository validation**

```bash
scripts/agent-validation.sh
pixi run -e dev cargo run --release -q -p decoder-tracker -- hash \
  --check crates/decoder-tracker/golden/decode-hashes.csv
```

Expected: both commands pass. The changed tracker/API tests and unchanged
golden hashes passed. Full workspace validation reaches pre-existing strict
clippy findings in `fornax-codec`, `fornax-dng` opcode/writer code, and
`fornax-raw-pipeline`; these are recorded as external failures.

- [x] **Step 2: Push the branch and run the full-corpus workflow**

```bash
gh workflow run decoder-tracker-corpus.yml --ref chore/full-corpus-metadata \
  -f filter='*/**/*' \
  -f run_duckdb_tests=false
```

The first run produced all complete shard artifacts, then failed in aggregation
because the committed baseline still used the legacy schema. After the guarded
migration fix, the user waived a workflow rerun and approved aggregating the
existing shard artifacts locally.

- [x] **Step 3: Review the migration**

Verify:

- all 2,020 keys remain present;
- all original baseline columns are unchanged;
- DNG metadata matches the previous 262-row sidecar;
- every fully passing row has complete source metadata;
- unsupported formats remain ordinary `unknown` metadata rows.

- [x] **Step 4: Apply the complete reviewed baseline**

```bash
pixi run -e tracker bash crates/decoder-tracker/scripts/update-badge.sh
git add crates/decoder-tracker/data/baseline.csv crates/decoder-tracker/README.md
git commit -m "chore: refresh corpus baseline with source encoding metadata"
git push
```

- [x] **Step 5: Re-run the workflow**

Waived by the user after the full shard set was already available. The reviewed
2,020-row artifact was aggregated locally with the fixed migration path.

---

## Files Expected To Change

- `crates/fornax-raw-core/`: normalized source metadata contract.
- `crates/fornax-dng/` and `crates/fornax-raw-*/`: source-layout mappings.
- `crates/fornax-raw-decoder/`: facade dispatch.
- `crates/decoder-tracker/`: schema, stages, CSV, SQL, audit, tests, data.
- `.github/workflows/decoder-tracker-corpus.yml`: remove duplicate metadata run.
- `.agents/skills/decoder-corpus-tracker/`, `crates/decoder-tracker/AGENTS.md`,
  `docs/decoder/corpus.md`: updated commands and schema.

## Validation

```bash
cargo test -p decoder-tracker
pixi run -e tracker cargo test -q -p decoder-tracker duckdb_
pixi run -e dev cargo run --release -q -p decoder-tracker -- hash \
  --check crates/decoder-tracker/golden/decode-hashes.csv
scripts/agent-validation.sh
```

Run the manual full-corpus workflow twice: the first result supplies the
reviewed schema migration, and the second must report unchanged.

## Decisions and Notes

- One canonical CSV replaces the sidecar; JSON is not needed.
- All formats share normalized fields. Numeric IDs are source-native and scoped
  by `format`; `compression_name` disambiguates variants.
- Formats without a decoder retain a baseline row with unknown metadata.
- The final golden set contains one lowest-pixel representative for each
  known-lossless encoding group. CFA does not split an encoding group. Known
  lossy and ambiguous codecs are excluded.
- `compression_lossless` is a `0`/`1` baseline column for constructed decoders.
  It is empty for unknown/undecoded rows. Ambiguous JPEG XL and WebP streams are
  conservatively classified as not proven lossless. Provenance is recorded in
  `docs/reference/codecs/lossless-classification.md`.
- No new dependency or workflow is introduced.
