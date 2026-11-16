# Fornax low-effort decoder fixes

## Goal

Fix the two known correctness defects from the Fornax versus FlashRaw report,
then remove the dominant serial-copy cost from large uncompressed Sony ARW and
Nikon NEF decoding without changing public APIs or expanding optimization scope.

## Scope

In scope:

- Select uncompressed Sony words from the actual strip storage layout, while
  retaining the packed 12-bit path for genuinely packed files.
- Time-box the Pentax K-1 lossless-JPEG mismatch and fix it only if the root
  cause is small and local.
- Memory-map decoder input through the existing TIFF source helper when the
  `mmap` feature is available, retaining the read fallback.
- Row-parallel, direct unpacking of 16-bit uncompressed Sony and Nikon words
  into the existing large-allocation output.
- Exact-output regression coverage, focused benchmarks, and a dated addendum to
  the existing Fornax versus FlashRaw report.

Out of scope:

- NEF Huffman, Canon CRX Rice, four-component Sony LJPEG, SIMD, allocator
  experiments, and DNG LJPEG tuning.
- Lossy formats and modes.
- Production API changes beyond the minimum internal data needed by the
  existing implementation.
- New unsafe or platform-specific code beyond the existing mmap helper.

## Task checklist

- [x] Record the baseline hashes and the five uncompressed probe results.
- [x] Investigate Sony uncompressed storage selection and Pentax LJPEG headers.
- [x] Add failing tests for the Sony word-width selection and the confirmed
  Pentax root cause.
- [x] Commit the Sony correctness fix independently.
- [x] Commit the Pentax fix independently if the cause is small and confirmed.
- [x] Add failing row-parallel unpack tests, then implement direct mmap-backed
  word unpacking for Sony and Nikon.
- [x] Verify all probe hashes and rerun the uncompressed comparison.
- [x] Append the dated before/after section to the existing performance report.
- [ ] Run focused and workspace validation, review the diff, and hand off one
  pull request.

## Files expected to change

- `crates/fornax-raw-sony/`
- `crates/fornax-raw-nikon/`
- `crates/fornax-tiff/`
- `crates/fornax-codec/`
- `crates/fornax-dng/` only if the shared uncompressed helper applies without
  behavior changes
- `crates/fornax-raw-decoder/` tests or benchmarks
- `crates/decoder-tracker/data/baseline.csv` for corrected hashes only
- `docs/reference/decoders/perf/flashraw-vs-fornax.md`
- `docs/plans/chore-fornax-low-effort-fixes.md`

## Validation

- Focused failing-then-passing tests for each corrected behavior and row
  unpack edge case.
- Raw mosaic hash comparison for all 14 probe samples.
- Probe runs at 1 and 16 threads for Sony ILCE-1, Nikon D500, Nikon D850,
  Sigma fp, and Leica SL2, using three process batches, two warmups, and five
  rounds.
- `scripts/agent-validation.sh` from the worktree, plus `git diff --check`.
- Profile remaining uncompressed bottlenecks if the measured gain is much
  smaller than expected.

## Decisions and notes

- RAW fixtures are hard-linked from the main checkout and remain read-only.
- Correctness commits precede the performance commit so each can be reverted
  independently.
- The existing report is updated in place; historical measurements remain
  clearly labeled.
- A confirmed Pentax root cause that requires broad LJPEG redesign is documented
  instead of implemented, as required by the time box.
- The Pentax LJPEG stream itself matches the direct FlashRaw and Rawler codec
  output. The mismatch came from selecting the last of four equal-sized
  full-resolution DNG IFDs; selecting the first resolves it locally.
- The shared word-row helper handles full-width 16-bit strips only. It is used
  by DNG 16-bit uncompressed data when that shape applies, but DNG packed
  12-bit data and all other compression paths keep their previous code.
- Sony 16-thread time remains memory-bound and does not improve over one
  thread. Nikon word decoding scales strongly and is slightly faster than
  FlashRaw in the final two-sample comparison.
