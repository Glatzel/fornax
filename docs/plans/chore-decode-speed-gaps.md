# Decode Speed Gap Plan

## Goal

Bring every requested Sony, Nikon, and Canon raw-decode row within its stated
FlashRaw 0.3.1 scalar-CPU ratio at one and sixteen threads without changing
output bytes or weakening corruption handling.

## Scope

In scope:

- Sony: ILCE-1, ILCE-7RM5, and ILCE-9M3 uncompressed and LJPEG.
- Nikon: Z5 II, D850, Z7, and Z9 Huffman; D6 and Z7 mode-10 packed.
- Canon: R5, R6, and R7 lossless CRX.
- Profile-driven, independently measured safe-Rust kernel and scheduling
  changes, plus the two required Phase 2 design notes and throwaway prototypes.

Out of scope:

- Public API changes, demosaic, unrelated crates, unsafe code, explicit SIMD
  intrinsics, copied decoder source, X-Trans/non-Bayer claims, and production
  Canon speculative starts without a verifiable checkpoint.

## Assumptions

- Sony "1.0~" parity means at most `1.1x` FlashRaw at both 1T and 16T; faster
  than FlashRaw is acceptable.
- Nikon and Canon targets require every listed row below `1.5x` at both 1T and
  16T.
- The target matrix uses the current performance manifest rows plus the
  separately measured Nikon Z5 II and original Canon R5 rows used by the
  current report.
- Benchmark comparison is `Decoder::decode` plus `raw_mosaic` against
  FlashRaw `open_path` plus `decode`, with scalar CPU FlashRaw, strict
  corruption handling, no crop, and no DNG opcodes.
- A change is retained only when its paired gain exceeds control drift and at
  least one requested row improves beyond noise.
- Phase 2 is a design/prototype gate. Stop before landing speculative parallel
  production code.

## Baseline

Phase 0 used the release-profile probe with `lto=thin`, `codegen-units=1`, warm
cache, two warmups, five rounds, three process batches per thread count, and
alternating Fornax/FlashRaw process order. All probe hashes matched the golden
reference. Baseline targets (`Fornax / FlashRaw`):

| Family | 1T | 16T |
| --- | ---: | ---: |
| Sony ILCE-1 uncompressed | 1.00x | 2.04x |
| Sony ILCE-1 LJPEG | 1.50x | 1.02x |
| Sony ILCE-7RM5 uncompressed | 1.00x | 1.94x |
| Sony ILCE-7RM5 LJPEG | 1.63x | 1.01x |
| Sony ILCE-9M3 uncompressed | 1.01x | 1.15x |
| Sony ILCE-9M3 LJPEG | 1.30x | 1.49x |
| Nikon Z5 II Huffman | 1.01x | 2.78x |
| Nikon D850 Huffman | 1.36x | 3.22x |
| Nikon Z7 Huffman | 1.46x | 3.82x |
| Nikon Z9 Huffman | 1.47x | 3.71x |
| Nikon D6 mode-10 packed | 4.19x | 1.17x |
| Nikon Z7 mode-10 packed | 4.62x | 1.32x |
| Canon R5 CRX | 2.35x | 3.00x |
| Canon R6 CRX | 2.30x | 3.14x |
| Canon R7 CRX | 2.52x | 3.45x |

The measured CRX utilization is about 2.2-2.5 CPUs at 16T because each file has
four independently coded plane work units. R5/R6/R7 each contain one tile, four
planes, one subband per plane, zero transform levels, and known subband sizes
and offsets. A plane's MED/Rice row decode is sequential because each line
depends on the preceding line; placement can be fused into each decoded line.

Nikon Huffman is one strip with no restart markers. Predictors use
`vertical[row parity][column]` for the first two columns and then a
same-row/two-column horizontal state. The fused decoder writes curve-mapped
predictors directly. Difference symbols can be extracted before prediction;
the predictor recurrence itself is still row-dependent and is not safely
row-parallel without a wavefront or sequential reconstruction.

Sony uncompressed at 16T is copy-bound. Input-map page faults are distributed
across all 16 Rayon workers (A1: 1,817 of 2,177 sampled faults; 7RM5: 1,666 of
1,893), so a single-thread pre-fault is not supported by the evidence. Sony
LJPEG's four-component fast scan is the dominant kernel; staging is inlined
under the emitted Rayon closure.

## Task Checklist

- [x] Task 1, Nikon mode-10: add a fixed-size 14-bit chunk test against the
  existing reference oracle, implement safe fixed-width chunk extraction, and
  retain only a measured win.
- [x] Task 2, Canon CRX: use the profile to make one Rice/line-kernel change at
  a time, retaining only independent measured wins; stop after two changes if
  combined 1T gain is below 10%.
- [x] Task 3, Sony uncompressed: A/B task/chunk sizing and mapping advice
  against the page-fault evidence; keep only a reproducible 16T win.
- [x] Task 4, Sony LJPEG: time-box two fast-window/placement/scheduling
  experiments; report DNG LJPEG regression controls.
- [x] Task 5, Phase 2A: write a design note and throwaway Nikon Huffman
  speculative prototype; measure real-file resynchronization and 8T/16T upside.
- [x] Task 6, Phase 2B: if Canon utilization remains low, write a design note
  and throwaway CRX parallel prototype covering the predictor dependencies and
  output exactness.
- [x] Task 7, Nikon Phase 2 production: land the reviewed speculative decode
  with exact-boundary merge, deterministic fallback, and row-parallel predictor.
- [x] Task 8, docs: update the current results, open priorities, reusable future
  facts, and one-line rejected-experiment entries.

## Files Expected to Change

- `crates/fornax-raw-decoder/src/decoders/nikon/codec.rs`
- `crates/fornax-raw-decoder/src/decoders/canon/codec/mod.rs`
- `crates/fornax-codec/src/samples.rs` only if the Sony uncompressed evidence
  supports a shared row-copy scheduling change.
- `crates/fornax-codec/src/ljpeg.rs` for the Sony/DNG shared LJPEG kernel, if
  the time-boxed experiment succeeds.
- `crates/fornax-tiff/src/lib.rs` only if input mapping advice is the retained
  Sony fix.
- `docs/plans/` for the two Phase 2 design notes.
- `docs/reference/decoders/perf/flashraw-vs-fornax.md` in the final docs commit.

## Validation

- Golden test and perf manifest hashes must remain unchanged after every code
  commit.
- Kernel equivalence tests use synthetic data and the preserved old/reference
  implementation, including partial groups, tiny images, row/column edges,
  truncation, and bit flips.
- `scripts/agent-validation.sh` after each commit.
- Interleaved 10-alternation A/B with two warmups at 1T, 8T, and 16T; report
  paired difference, range width, faults, RSS, and controls.
- Compare compile time and flag any row above `1.5x` memory growth.

## Decisions and Notes

- The Phase 0 measurements and profiles are local evidence under
  `target/analysis/decode-speed-gaps/`; host timing data is not committed.
- The shared `external/raw.pixls.us` LFS materialization is hard-linked and must
  never be staged or committed.
- No production vector intrinsics or new unsafe code are allowed.
- A missing target is reported with its measured floor and a concrete next
  design step rather than replaced with an unverified claim.
- Multi-thread performance takes priority over a one-thread-only optimization.
  Allocation policy is selected by allocation and write behavior, never by the
  active Rayon thread count.
