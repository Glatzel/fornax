# DNG Compression Coverage and Corpus Status Plan

## Goal

Document the DNG decoder and payload-layout gaps exposed by the pinned corpus,
record the decision that zstd and future codecs are mandatory build
capabilities, and make the decoder tracker README publish one aggregate corpus
badge plus a per-format `passed/total` table.

## Scope

In scope:

- Document compression identifiers `1`, `7`, `8`, `34892`, `50000`, `50002`,
  and `52546`, including the legacy TIFF JPEG XL identifier `50002` and the DNG
  1.7 JPEG XL identifier `52546`.
- Separate missing codecs and identifier mappings from unsupported
  multi-channel `LinearRaw`, lossless-JPEG tile layouts, CFA layouts, and TIFF
  metadata/layout failures.
- Record the approved policy that zstd and every future codec are always
  compiled and have no per-codec Cargo features. Mark the current zstd feature
  gate as a pending implementation gap because this patch does not change
  Cargo manifests or Rust `cfg` gates.
- Generate an accumulated badge and a per-format status table in the
  `decoder-tracker` README from `baseline.csv`.
- Add the required per-work plan and update related Markdown documentation.

Out of scope:

- Adding compression decoders, including legacy JPEG XL identifier `50002`.
- Removing the current zstd Cargo feature or changing Rust feature gates.
- Changing the baseline, manifest, tracker CSV schema, workflow, or tests.
- Decoding multi-channel `LinearRaw` through the mono `RawMosaic` interface.
- Adding generic TIFF-only codecs such as LZW, old-style JPEG, or PackBits.

## Task Checklist

- [x] Create the branch and worktree from `origin/main`.
- [x] Update DNG, TIFF, codec, architecture, reference, and corpus Markdown
  with compression coverage and failure taxonomy.
- [x] Update `update-badge.sh` to derive accumulated and per-format counts from
  `baseline.csv`.
- [x] Regenerate the `decoder-tracker` README badge and status table.
- [x] Run focused shell, DuckDB, tracker-test, and Markdown checks.
- [x] Run repository validation and inspect the final diff.
- [x] Commit, push, open one pull request, and report its URL and validation.

## Files Expected to Change

- `crates/decoder-tracker/scripts/update-badge.sh`
- `crates/decoder-tracker/README.md`
- `crates/fornax-tiff/README.md`
- `crates/fornax-codec/README.md`
- `crates/fornax-dng/README.md`
- `docs/architecture.md`
- `docs/decoder/architecture.md`
- `docs/decoder/corpus.md`
- `docs/decoder/formats/dng.md`
- `docs/references/dng-compression.md`
- `docs/references/dng-lossless-jpeg.md`
- `docs/plans/chore-dng-compression-coverage.md`

## Validation

- `bash -n crates/decoder-tracker/scripts/update-badge.sh`
- Run `update-badge.sh` twice against the committed baseline and require an
  idempotent result with `209/262` overall and a `DNG` row.
- Run the script against a temporary mixed-format baseline and require
  independently correct format rows, an accumulated badge, and a generic
  corpus badge label.
- `cargo test -p decoder-tracker duckdb_badge_script_updates_pass_and_total_counts`
- `scripts/agent-validation.sh`
- `git diff --check`

Do not run Markdown lint locally; CI owns it.

Validation record:

- `bash -n crates/decoder-tracker/scripts/update-badge.sh`, the existing badge
  regression test, actual-baseline idempotence, and a mixed-format temporary
  baseline passed.
- `scripts/agent-validation.sh --docs-only` passed.
- `scripts/agent-validation.sh` passed tracker tests, workspace tests,
  doctests, formatting, and rustdoc, then failed in clippy on pre-existing
  `unseparated_literal_suffix` warnings at
  `crates/fornax-core/src/metadata.rs:750-751`. This branch does not modify
  `fornax-core`.
- Pull request: <https://github.com/Glatzel/fornax/pull/675>.

## Decisions and Notes

- The accepted baseline currently has `209/262` DNG rows passing all stages.
- Compression `50002` is the legacy TIFF JPEG XL identifier and is not
  recognized by `fornax-tiff` today. Compression `52546` is the DNG 1.7 JPEG XL
  identifier and is mapped to the single-channel 8-16-bit JPEG XL decoder.
- Compression `50000` is experimental Zstandard and currently requires the
  `zstd` feature. The target architecture removes per-codec features, but that
  Cargo and Rust migration is outside this patch.
- Lossless JPEG-92 samples using `0x10` Huffman selectors or multi-component
  half-height/subsampled layouts remain unsupported.
- Three-sample `LinearRaw` payloads using compression `1`, `7`, or `34892`
  fail before codec dispatch because the active decoder contract requires one
  raw sample per pixel.
- Missing `RowsPerStrip`, truncated chunks, and chunk-count mismatches are TIFF
  layout or parser gaps, not evidence of missing compression decoders.
- The badge label remains `DNG corpus` while DNG is the only tracked format and
  becomes `RAW corpus` when more formats are present. This keeps the existing
  tracker test valid without modifying test code.

## Task 1: Document Compression Coverage

**Files:**

- Modify: `crates/fornax-tiff/README.md`
- Modify: `crates/fornax-codec/README.md`
- Modify: `crates/fornax-dng/README.md`
- Modify: `docs/architecture.md`
- Modify: `docs/decoder/architecture.md`
- Modify: `docs/decoder/corpus.md`
- Modify: `docs/decoder/formats/dng.md`
- Modify: `docs/references/dng-compression.md`
- Modify: `docs/references/dng-lossless-jpeg.md`

Add the shared compression/identifier matrix, mandatory-codec target, current
zstd feature gap, lossless-JPEG limitations, multi-sample `LinearRaw`
limitation, and non-codec failure taxonomy described above. Do not claim that
the mandatory-codec migration or legacy JPEG XL `50002` support is implemented.

## Task 2: Generate Per-Format Corpus Status

**Files:**

- Modify: `crates/decoder-tracker/scripts/update-badge.sh`
- Modify: `crates/decoder-tracker/README.md`

Replace the marker block with one aggregate badge and a deterministic
`Format | Status` table. Aggregate all rows for the badge, group rows by format
for the table, use `passed/total` status values, validate all counts, and keep
the replacement atomic and idempotent. Preserve the existing badge label while
only DNG is tracked and use `RAW corpus` when multiple formats exist.

## Task 3: Verify and Hand Off

**Files:**

- Verify only.

Run the focused checks, repository validation, and final diff audit listed in
the Validation section. Commit the Markdown and script changes, push the
branch, and open one chore pull request.
