# Nikon Huffman and mode-10 decode speedups

## Goal

Close the one-thread Nikon Huffman decode gap with a fused 13-bit
Huffman/difference table and a 64-bit row window, and make Nikon mode-10 packed
rows scale by parallelizing row unpacking into the final sample buffer.

## Scope

In scope:

- Lossless Nikon mode-3 Huffman at 12 and 14 bits.
- A fused lookup for samples whose code and difference fit in 13 bits.
- An exact, safe 64-bit window fast path with the existing decoder retained as
  the fallback and test oracle.
- Direct final-row output with the current predictor, curve, first-row, and
  first-column semantics.
- Nikon mode-10 14-bit little-endian packed rows, including padded row strides.
- Reuse of the mapped TIFF source, shared row-unpack helper, four-million
  sample cutoff, and existing output allocation policy.
- Synthetic equivalence, truncation, invalid-stream, and geometry tests.
- Golden hash verification, focused and repository validation, interleaved
  release-profile A/B measurements, profiling, and the existing performance
  report.
- Removal of `Nikon/D850/Nikon-D850-14bit-uncompressed.NEF` from the
  performance manifest.

Out of scope:

- Parallel or speculative Huffman entropy decoding.
- SIMD, new unsafe code, lossy/HE modes, mode-2/6/9 packed changes, LJPEG,
  CRX, DNG, or demosaic.
- Public decoder output or metadata changes.
- RAW payloads, run logs, or generated benchmark data in commits.

## Task checklist

- [x] Record the current Nikon table, predictor, strip, packed-row, and
  reference behavior with file line evidence.
- [x] Record the pre-change 1T/16T timings, faults, FlashRaw comparisons, and
  D850 1T perf profile.
- [x] Add failing synthetic Huffman tests for fast-vs-slow equivalence at 12
  and 14 bits, odd widths, one-row/one-column images, long-code fallback, large
  differences, and exact/truncated end positions.
- [x] Implement the fused table and 64-bit row kernel with the old path as the
  fallback; run focused tests and all Nikon NEF golden hashes.
- [x] Commit the independently revertible Huffman change.
- [x] Add failing mode-10 tests for odd widths/heights, one row, chunk
  boundaries, padded strides, and truncation.
- [x] Route mode-10 packed rows through the shared direct row helper and commit
  the independently revertible parallelism change.
- [x] Remove the D850 uncompressed NEF from the performance manifest and update
  the benchmark matrix.
- [x] Run the full Nikon and relevant control golden hashes, validation, and
  release-profile interleaved A/B measurements.
- [x] Append the dated methods, before/after tables, profile, follow-up strip
  analysis, and residual-gap explanation to the existing FlashRaw comparison.
- [x] Commit documentation last and inspect all three commits without pushing.

## Files expected to change

- `crates/fornax-raw-nikon/src/codec.rs`
- `crates/fornax-raw-nikon/src/codec_contract_tests.rs`
- `crates/fornax-codec/src/samples.rs` only if the shared row helper needs a
  narrowly scoped test/visibility adjustment
- `crates/decoder-tracker/golden/perf-manifest.csv`
- `docs/reference/decoders/perf/data/golden-benchmark-matrix.csv`
- `docs/reference/decoders/perf/flashraw-vs-fornax.md`
- `docs/plans/chore-nikon-decode-speedups.md`

## Validation

- `cargo test -p fornax-raw-nikon`
- `cargo test -p fornax-codec`
- `cargo test -p fornax-raw-decoder`
- `cargo test -p decoder-tracker`
- Golden `hash --check` for the complete test and performance hashes.
- Focused real-file hashes for every Nikon Huffman and mode-10 manifest input.
- Interleaved release-profile A/B, ten alternations, two warmups, one and
  sixteen threads, with Sony LJPEG, Canon CRX, and DNG controls.
- `scripts/agent-validation.sh`
- `git diff --check`

## Decisions and notes

- The user supplied a detailed implementation spec and requested assumptions
  instead of design questions. The implementation follows that spec directly.
- Start at a 13-bit fused index. Each entry uses one `u32` (difference in the
  low 16 bits, total consumed bits in bits 16-23, and an escape marker), so one
  table is 32 KiB and is built once per decode, not per row.
- Fast groups reload one 64-bit MSB-aligned window and decode up to four
  samples when each fused entry consumes at most 13 bits. Long codes and large
  differences reload and use the existing exact reader path.
- The old row implementation remains callable in tests and as the fallback for
  streams that cannot provide a full window.
- The current lossless Nikon curve is identity in production, but the fast path
  still performs the same checked curve lookup rather than assuming identity.
- Mode-10 packed samples are 14-bit little-endian, group width 8 samples / 14
  bytes, while actual row strides are 16-byte aligned. The shared helper gets
  the physical stride through a row source closure and handles non-aligned
  widths plus the partial tail group.
- The shared four-million-sample/rayon cutoff is retained; current Nikon
  targets are above it and small synthetic inputs remain serial.
- Rawler is LGPL-2.1 and LibRaw is LGPL-2.1 or CDDL-1.0; they are behavior-only
  references. FlashRaw 0.3.1 is MIT OR Apache-2.0. No reference source is
  copied.
- The full validation script passed after the linked worktree's ignored
  submodule gitfiles were pointed at the existing main-checkout gitdirs.
  Standalone golden checks that first encountered the copied path were repeated
  with the main checkout's valid corpus path before the full script passed.
