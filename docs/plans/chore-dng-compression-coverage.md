# DNG Compression Coverage and Corpus Status Plan

## Goal

Document and fix the DNG decoder gaps exposed by the pinned corpus, add legacy
TIFF JPEG XL identifier support, make zstd and future codecs mandatory build
capabilities, and make the decoder tracker README publish one aggregate corpus
badge plus a per-format `passed/total` table.

## Scope

In scope:

- Document compression identifiers `1`, `7`, `8`, `34892`, `50000`, `50002`,
  and `52546`, including the legacy TIFF JPEG XL identifier `50002` and the DNG
  1.7 JPEG XL identifier `52546`.
- Map both JPEG XL identifiers to the existing JPEG XL decoder.
- Separate missing codecs and identifier mappings from unsupported
  multi-channel `LinearRaw`, lossless-JPEG tile layouts, CFA layouts, and TIFF
  metadata/layout failures.
- Make zstd a mandatory dependency and remove all zstd feature gates and
  feature-specific fallback behavior.
- Record that every future codec is always compiled and has no per-codec Cargo
  feature.
- Generate an accumulated badge and a per-format status table in the
  `decoder-tracker` README from `baseline.csv`.
- Add the required per-work plan and update related Markdown documentation.

Out of scope:

- Adding lossless-JPEG variants or multi-channel `LinearRaw` decoding.
- Manually changing the baseline; CI's manual corpus workflow updates it.
- Changing the manifest or tracker CSV schema.
- Decoding multi-channel `LinearRaw` through the mono `RawMosaic` interface.
- Adding generic TIFF-only codecs such as LZW, old-style JPEG, or PackBits.

## Task Checklist

- [x] Create the branch and worktree from `origin/main`.
- [x] Update DNG, TIFF, codec, architecture, reference, and corpus Markdown
  with compression coverage and failure taxonomy.
- [x] Update `update-badge.sh` to derive accumulated and per-format counts from
  `baseline.csv`.
- [x] Regenerate the `decoder-tracker` README badge and status table.
- [x] Add failing identifier and feature-independence tests.
- [x] Map legacy JPEG XL identifier `50002` to the JPEG XL decoder.
- [x] Make zstd mandatory and remove zstd feature gates from code, tests, docs,
  and CI.
- [x] Run focused shell, DuckDB, tracker-test, and Markdown checks.
- [x] Run repository validation and inspect the final diff.
- [x] Commit, push, open one pull request, and report its URL and validation.
- [ ] Run the manual corpus workflow and let it update the baseline.

## Files Expected to Change

- `crates/decoder-tracker/scripts/update-badge.sh`
- `crates/decoder-tracker/README.md`
- `.github/workflows/ci.yml`
- `crates/fornax-cli/Cargo.toml`
- `crates/fornax/Cargo.toml`
- `crates/fornax-codec/Cargo.toml`
- `crates/fornax-codec/src/error.rs`
- `crates/fornax-codec/src/zstd.rs`
- `crates/fornax-codec/tests/decode_contract.rs`
- `crates/fornax-codec/tests/encode_contract.rs`
- `crates/fornax-dng/Cargo.toml`
- `crates/fornax-dng/benches/writer.rs`
- `crates/fornax-dng/src/dng_decoder/mod.rs`
- `crates/fornax-dng/src/writer.rs`
- `crates/fornax-tiff/README.md`
- `crates/fornax-tiff/src/constants.rs`
- `crates/fornax-tiff/tests/constants.rs`
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
- `cargo test -p fornax-tiff compression_methods_preserve_known_and_unknown_identifiers`
- `cargo test -p fornax-dng legacy_jpeg_xl_metadata_maps_to_jpeg_xl_decode_compression`
- `cargo test -p fornax-codec zstd_round_trips_integer_and_float_chunks`
- `cargo test -p fornax-dng zstd_writer_round_trips_u16_samples`
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
  `crates/fornax-core/src/metadata.rs:750-751` and a pre-existing
  `suboptimal_flops` warning at `crates/fornax-postprocessor/src/lib.rs:415`.
  This branch does not modify those crates.
- Pull request: <https://github.com/Glatzel/fornax/pull/675>.

## Decisions and Notes

- The accepted baseline currently has `209/262` DNG rows passing all stages.
- Compression `50002` is the legacy TIFF JPEG XL identifier and `52546` is the
  DNG 1.7 JPEG XL identifier. Both map to the single-channel 8-16-bit JPEG XL
  decoder.
- Compression `50000` is experimental Zstandard and is always compiled using
  pure-Rust `structured-zstd`. No codec-specific Cargo feature remains.
- Lossless JPEG-92 samples using `0x10` Huffman selectors or multi-component
  half-height/subsampled layouts remain unsupported.
- Three-sample `LinearRaw` payloads using compression `1`, `7`, or `34892`
  fail before codec dispatch because the active decoder contract requires one
  raw sample per pixel.
- Missing `RowsPerStrip`, truncated chunks, and chunk-count mismatches are TIFF
  layout or parser gaps, not evidence of missing compression decoders.
- The badge slug is `raw.pixls.us_corpus_pass`, rendering the badge label
  `raw.pixls.us corpus pass`; the badge always reports the accumulated pass
  count while the table reports each format separately.
- Legacy and DNG 1.7 JPEG XL identifiers share one decode path.
- `structured-zstd` is a mandatory dependency. There is no `zstd` crate
  feature, feature-gated fallback, or feature-specific benchmark option.

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

Add the shared compression/identifier matrix, lossless-JPEG limitations,
multi-sample `LinearRaw` limitation, and non-codec failure taxonomy described
above. Update the matrix again after the implementation tasks make zstd and
legacy JPEG XL `50002` support unconditional.

## Task 2: Generate Per-Format Corpus Status

**Files:**

- Modify: `crates/decoder-tracker/scripts/update-badge.sh`
- Modify: `crates/decoder-tracker/README.md`

Replace the marker block with one aggregate badge and a deterministic
`Format | Status` table. Aggregate all rows for the badge, group rows by format
for the table, use `passed/total` status values, validate all counts, and keep
the replacement atomic and idempotent. Use the badge slug
`raw.pixls.us_corpus_pass`.

## Task 3: Verify and Hand Off

**Files:**

- Verify only.

Run the focused checks, repository validation, and final diff audit listed in
the Validation section. Commit the Markdown and script changes, push the
branch, and open one chore pull request.

## Task 4: Support Legacy JPEG XL and Mandatory Zstandard

Add `CompressionMethod::JpegXlLegacy` for identifier `50002` and route both
JPEG XL identifiers to `DecodeCompression::JpegXl`. Make `structured-zstd`
non-optional, remove every zstd `cfg` gate and fallback, remove the `zstd`
feature from workspace crates and the CLI dependency, and remove the
feature-specific benchmark option from CI.

Test-first steps:

- Add `50002` to the TIFF identifier round-trip test and verify it fails before
  adding `CompressionMethod::JpegXlLegacy`.
- Add a DNG decode-plan test proving `50002` maps to
  `DecodeCompression::JpegXl` and verify it fails before adding the match arm.
- Ungate zstd round-trip tests and verify they fail without the feature before
  making `structured-zstd` mandatory.
- Remove feature-specific fallback tests and gates after the mandatory path
  passes.

Completion evidence:

- `cargo test -p fornax-tiff compression_methods_preserve_known_and_unknown_identifiers`
- `cargo test -p fornax-dng legacy_jpeg_xl_metadata_maps_to_jpeg_xl_decode_compression`
- `cargo test -p fornax-codec zstd_round_trips_integer_and_float_chunks`
- `cargo test -p fornax-dng zstd_writer_round_trips_u16_samples`
- `cargo test -p fornax-tiff -p fornax-codec -p fornax-dng -p fornax-cli -p fornax`
- `cargo test --workspace --all-features --exclude compare-demosaic`
- `cargo bench -p fornax-dng --bench writer --no-run`
- `cargo clippy -p fornax-tiff -p fornax-codec -p fornax-dng --all-targets
  --all-features -- -A clippy::too_many_arguments -D warnings`
- `RUSTDOCFLAGS='-D warnings' cargo doc -p fornax-tiff -p fornax-codec
  -p fornax-dng -p fornax-cli -p fornax --all-features --no-deps`

## Task 5: Refresh the Corpus Through CI

Do not edit `baseline.csv` locally. Run the manual decoder-tracker corpus
workflow after the implementation is pushed, then commit the generated
baseline/diff review changes through the workflow's normal review process.
