# Eliminate uncompressed and lossless JPEG writer regressions

## Goal

Restore DNG writer performance for Uncompressed and Lossless JPEG after the
`fornax-codec` extraction without changing public codec interfaces, encoded
payload semantics, or DNG output geometry. Both codecs must stay within the
local CodSpeed replacement gates relative to `0bd98a9` and the pre-fix branch.

## Scope

In scope:

- Add a whole-image serialization fast path to `UncompressedCodec` when one
  request chunk covers the complete image.
- Replace Lossless JPEG's generic byte serialization with direct extraction
  from `RawMosaic` into its reusable tile sample buffer.
- Add U8/U16/U32/F32 and byte-order coverage for whole-image Uncompressed
  serialization.
- Add U8/U16 Lossless JPEG round-trip coverage with partial edge tiles.
- Run codec and DNG tests, local writer benchmarks, and repository validation.

Out of scope:

- Public `ICodecEncode`/`ICodecDecode`, request, result, feature, or dependency
  changes.
- Deflate, Zstandard, or other codec writer changes.
- DNG tag, tile geometry, JPEG Huffman, file-size, or byte-order changes.
- `TODO.md` changes, waivers, or acknowledged benchmark regressions.

## Task checklist

- [x] Record local baseline and pre-fix medians for the six Uncompressed and
  Lossless JPEG writer cases.
- [x] Add failure-first whole-image Uncompressed serialization tests for U8,
  U16, U32, and F32 in both byte orders, then implement the private fast path.
- [x] Add failure-first Lossless JPEG partial-edge-tile round trips for U8 and
  U16, then replace generic serialization with direct tile extraction.
- [x] Re-run focused tests and alternating post-fix benchmarks; require every
  post-fix median to be at most `1.05 * baseline` and each previously regressed
  case to be at most `0.90 * pre-fix`.
- [ ] Run `scripts/agent-validation.sh`, inspect the final diff, commit, and
  push the same branch to PR #660.

## Files expected to change

- `crates/fornax-codec/src/uncompressed.rs`
- `crates/fornax-codec/src/ljpeg.rs`
- `crates/fornax-codec/tests/encode_contract.rs`
- `docs/plans/chore-codec-writer-perf.md`
- `typos.toml` (normalize the branch's CRLF line endings so the required
  validation diff check accepts the existing `Flate` identifier exception)

## Validation

Run from the worktree root:

```bash
cargo -q test -p fornax-codec --all-features
cargo -q test -p fornax-codec
cargo -q test -p fornax-dng --all-features
cargo -q test -p fornax-dng
```

Benchmark the `0bd98a9` baseline, the pre-fix branch, and the post-fix branch
sequentially in three alternating rounds with separate target directories:

```bash
FORNAX_BENCH_DNG_COMPRESSION=uncompressed,lossless-jpeg
FORNAX_BENCH_SIZE=1000,2000
FORNAX_BENCH_DNG_RAW=/home/glatzel/project/fornax/.worktrees/chore-extract-fornax-codec/external/raw-images/images/colorchart-5D2-6000K.dng
OMP_NUM_THREADS=1
RAYON_NUM_THREADS=1
```

Read every case's `new/estimates.json` point estimate. Do not push unless all
six post-fix medians are within `5%` of baseline and all five cases regressed
by the extraction are at least `10%` faster than pre-fix. After those gates,
run:

```bash
scripts/agent-validation.sh
```

## Decisions and notes

- Ruling: keep the generic chunk serializer for Deflate and Zstandard because
  the reported regressions are limited to Uncompressed and Lossless JPEG, and
  changing unaffected codecs would broaden the benchmark surface.
- Ruling: keep all optimizations private to `fornax-codec`; the public contracts
  and the DNG writer's Little-endian request remain unchanged.
- Ruling: batch JPEG bit packing after the first post-fix round showed that
  direct tile extraction alone did not clear the synthetic Lossless JPEG gate.
  The optimized writer emits the same Huffman bytes, padding, and byte
  stuffing; benchmark output sizes remain unchanged.
- Ruling: write Uncompressed samples directly into reserved output storage
  after the first post-fix round showed that repeated `extend_from_slice` calls
  regressed the full-image path.
- Lossless JPEG edge tiles retain zero padding to the declared tile dimensions,
  matching the existing DNG bytes and decoder behavior.
- Local timing measurements are verification evidence, not committed baseline
  data.
