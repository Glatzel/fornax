# Decoder Performance Cleanup and Nikon/Canon Closure

## Goal

Make the current FlashRaw comparison report concise and accurate, then reduce
the remaining Nikon mode-10/Huffman and Canon CRX gaps with independently
measured, bit-exact changes in one four-commit branch.

## Scope

In scope:

- Remove dated and per-change narratives from the FlashRaw comparison while
  retaining current results, method, correctness, decisions, and archive links.
- Add a grouped Nikon packed-row kernel and keep the per-sample implementation
  as a test-only oracle.
- Profile Nikon Huffman, then measure the checked word reader, identity-curve
  fast path, and compact non-identity lookup separately.
- Profile Canon CRX, inspect FlashRaw's line-context/placement approach without
  copying it, and implement only profile-supported bit-exact changes.
- Verify Nikon NEF and Canon CR3 golden hashes, synthetic edge cases, strict
  corruption errors, release-profile A/B results, and repository validation.

Out of scope:

- Nikon speculative entropy parallelism and single-scan CRX parallelism.
- The opening-line vendor-crate consolidation. The four named commits are the
  authoritative deliverables and leave no independently revertible slot for it.
- Demosaic, new unsafe, SIMD, copied third-party code, push, merge, or PR.
- Behavior outside Nikon NEF and Canon CRX decode.

## Review Focus

- Nikon widths not divisible by eight, padded strides, partial groups, and
  short final strips.
- Nikon streams ending exactly after the last sample versus before it or inside
  a code; non-identity curves; long codes and large differences.
- CRX edge tiles, all four plane placements, truncated bands, and bit flips.
- Every candidate must be measured alone and exceed control drift.

## Task Checklist

### Task 1: Clean the active performance report

- [x] Rewrite the report around current state and fold final Nikon numbers into
  the main Results table.
- [x] Remove dated sections, base/new tables, per-run faults, and RSS tables;
  retain the one durable FlashRaw 16T Huffman RSS sentence.
- [x] Replace improvement priorities with ranked open work only.
- [x] Add durable Notes for future work, including Nikon strip/parity
  parallelism, fused-table geometry, mode-10 threshold, and FlashRaw wins.
- [x] Correct stale manifest/matrix counts and remove the D850 uncompressed
  narrative; leave dated CSVs unchanged.
- [x] Run the documentation link checker and commit docs plus this plan.

### Task 2: Nikon

- [x] Add failing grouped-kernel oracle tests for widths 1/5/8/9/17, one row,
  padded and non-byte-aligned rows, partial strips, and truncation.
- [x] Implement safe grouped 12/14-bit little-endian extraction and preserve
  strict truncation errors; measure the mode-10 kernel alone.
- [x] Add failing Huffman reader/path tests over both trees, identity and
  non-identity curves, exact/early/mid-code ends, long codes, large
  differences, truncation, and bit flips.
- [x] Measure the checked word reader and identity fast path separately; both
  regressed or were neutral and were dropped. The compact non-identity lookup
  is not supported by the production identity-curve profile.
- [x] Keep the 13-bit/32 KiB fused table and the shared 4,194,304-sample
  row-parallel threshold unless profile evidence says otherwise.
- [x] Run Nikon/codec/decoder tests and all Nikon golden hashes; commit accepted
  Nikon changes as commit 2.

### Task 3: Canon CRX

- [x] Profile R5/R6/R7 with `perf record` and `perf stat` at 1T and 16T; report
  the requested cycle split, instructions, branch misses, and minor faults.
- [x] Read FlashRaw 0.3.1's CRX line-context and plane-placement structure; its
  code is MIT OR Apache-2.0 but remains approach-only reference material.
- [x] Add failing synthetic CRX oracle, truncation, and bit-flip tests.
- [x] Measure fused Rice extraction and direct one-thread placement separately;
  both exceeded their paired variation. Stop at the two-change time box rather
  than stack compact-context or bounds-check changes without profile support.
- [x] Stop after two accepted profile-driven changes if R6 1T gain is below
  10%; record neutral/regressed attempts as rejected experiments.
- [ ] Run Canon/decoder tests and all Canon golden hashes; commit accepted CRX
  changes as commit 3.

### Task 4: Final documentation

- [x] Run complete golden hash verification and `scripts/agent-validation.sh`.
- [x] Record accepted measurements in a new dated CSV.
- [x] Update only Results, Format findings, Improvement priorities, Notes for
  future work, and rejected experiments.
- [x] Run the document link checker, inspect all four commits, and commit final
  docs as commit 4.

## Files Expected to Change

- `docs/reference/decoders/perf/flashraw-vs-fornax.md`
- `docs/reference/decoders/perf/data/` (new CSV only)
- `docs/plans/chore-decoder-perf-consolidation.md`
- `crates/fornax-raw-nikon/src/codec.rs`
- `crates/fornax-raw-nikon/src/codec_contract_tests.rs`
- `crates/fornax-raw-canon/src/codec/mod.rs`
- Benchmark/probe files only if needed to generate requested evidence.

## Validation

- `cargo test -p fornax-raw-nikon`
- `cargo test -p fornax-raw-canon`
- `cargo test -p fornax-codec`
- `cargo test -p fornax-raw-decoder`
- Golden `hash --check` for both manifests, plus focused Nikon/Canon rows.
- `python3 scripts/check-doc-links.py`
- `scripts/agent-validation.sh`
- Ten-alternation release-profile A/B at 1T/16T with DNG and Sony LJPEG controls.

## Decisions and Notes

- The user's detailed specification and instruction to proceed unless blocked
  replace design questions.
- Each sub-optimization must pass focused tests and be measured before another
  is stacked; neutral or regressed changes are dropped.
- The old Nikon packed and Huffman implementations remain test oracles.
- Truncated or invalid input is always an error, never zero padding.
- `external/raw.pixls.us` is read-only shared input and must never be committed.
