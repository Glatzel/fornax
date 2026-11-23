> Status: active plan.

# FlashRaw 0.4.0 decode-gap follow-up

## Goal

Close the next measured decoder gaps to FlashRaw 0.4.0, prioritizing 16-thread
wall time while preserving bit-exact output, strict corruption errors, public
APIs, and the existing crate boundaries.

## Scope

In scope:

- DNG OpcodeList1 row-band execution with per-worker scratch.
- DNG Lossless JPEG scheduler evidence for Pentax 645Z.
- Safe Canon CRX entropy/reconstruction separation and shared-kernel evidence.
- The CRX speculative-start design gate if 16T utilization remains low.

Out of scope:

- Deflate and Sony/Nikon kernel work until the CRX gate reports.
- Unsafe code, explicit SIMD intrinsics, new dependencies, or copied code.
- Single-thread-only paths or tuning selected by the Rayon thread count.
- Public API changes, demosaic changes, or unrelated decoder work.

## Task checklist

- [x] Phase 0: capture interleaved 1T/8T/16T baselines and required profiles.
- [x] Phase 0: census materialized DNG OpcodeList1 IDs and record the
  unmaterialized-corpus limitation.
- [x] Phase 1: implement and measure chunk-local OpcodeList1 execution.
- [x] Phase 2: establish whether Pentax work can be scheduled safely.
- [x] Phase 3: measure safe CRX changes and write the speculative design gate.
- [x] Phase 4: profile Deflate, test parallel tile reconstruction, and retain
  only passing changes.
- [x] Phase 5: profile and test two shared Sony/Nikon kernel changes.
- [x] Final: update current performance findings and rejected experiments.

## Phase 0 baseline and plan

All timings use the release profile, `lto=thin`, `codegen-units=1`, scalar
FlashRaw CPU, warm page cache, two warmups, three rounds, and three alternating
process batches. Ratio is Fornax / FlashRaw.

| Row | 1T | 8T | 16T | Goal | Dominant evidence | Planned change |
| --- | ---: | ---: | ---: | --- | --- | --- |
| G1 X | 8.20x | 11.20x | 17.28x | < 2x at 16T | Full-plane conversion 32%, opcode 25%, `roundf` 25% | L2 row bands, fused conversion/application/output |
| Nokia | 10.70x | 18.11x | 27.52x | < 2x at 16T | Opcode 41%, `roundf` 32%, conversion 19% | Same, plus four MapTable passes |
| Leica SL2 | 6.88x | 11.55x | 19.24x | < 2x at 16T | Conversion 36%, opcode 26%, `roundf` 24% | Same, neighborhood halo |
| Pentax 645Z | 1.24x | 2.31x | 2.49x | <= 1.36x | One 72,478,678-byte strip; entropy closure 88% | Evidence only; intra-scan is gated |
| R5 CRX | 2.17x | 3.04x | 2.63x | < 2x | Rice 44%, line prediction 47%, placement 6% | Safe reconstruction work first, then gate |
| R6 CRX | 2.18x | 2.94x | 2.90x | < 2x | Four planes, one tile | Same |
| R7 CRX | 2.37x | 3.90x | 3.20x | < 2x | Four planes, one tile | Same |
| Deflate FP24 | 1.39x | 1.80x | 1.24x | <= 1.0x | Inflate and float predictor | Phase 4, after CRX gate |
| Deflate FP32 | 1.24x | 1.59x | 1.18x | <= 1.0x | Inflate and float predictor | Phase 4, after CRX gate |
| Sony A1 LJPEG | 1.46x | 1.25x | 0.77x | keep 16T faster; 1T < 1.3x | Shared entropy kernel | Phase 5, after CRX gate |
| Sony 7RM5 LJPEG | 1.57x | 1.32x | 0.79x | keep 16T faster; 1T < 1.3x | Shared entropy kernel | Phase 5, after CRX gate |
| Nikon D6 mode-10 | 2.46x | 0.78x | 0.88x | 16T >= 10% faster; 1T < 1.5x | Packed row kernel | Phase 5, after CRX gate |
| Nikon Z7 mode-10 | 2.60x | 0.74x | 0.82x | 16T >= 10% faster; 1T < 1.5x | Packed row kernel | Phase 5, after CRX gate |

Named controls were also measured: 5D3 uncompressed, Deflate FP16, Sigma fp L
LJPEG, Nikon Z7 Huffman, and Sony 9M3 LJPEG. The six 5D3 HDR floating-point
files are the float controls. No raw.pixls.us pinned path or committed manifest
entry identifies a Gaussian DNG; that control remains unresolved.

The materialized DNG census found only two opcode IDs: FixBadPixelsConstant
(ID 4) in G1 X and Leica SL2, and MapTable (ID 7) in Nokia. The implementation
supports IDs 3-13 and rejects 1, 2, 6, and unknown operations. A source scan
found 199 additional DNG baseline rows whose LFS payloads are not materialized;
the committed census therefore covers 30 materialized successful DNG files.

## Files expected to change

- `crates/fornax-raw/src/dng/dng_decoder/transforms.rs`
- `crates/fornax-raw/src/dng/dng_decoder/opcodes.rs`
- `crates/fornax-raw/src/decoders/canon/codec/mod.rs` only if safe
  reconstruction work has measured value.
- `docs/plans/chore-flashraw-next-gaps.md`
- `docs/plans/chore-crx-intra-plane-start.md`
- `docs/reference/decoders/perf/flashraw-vs-fornax.md`

## Validation

- Every code commit: affected golden hashes, strict synthetic oracle tests,
  `scripts/agent-validation.sh`, and compiler timing.
- Every retained change: interleaved base/new measurements at 1T, 8T, and 16T
  with controls, paired ranges, faults, RSS, and CPU utilization.
- Every rejected change: one line in the performance document's rejected
  experiments section.
- Final documentation: link check and docs-only validation.

## Decisions and notes

- Opcode parsing is cached once per `raw_mosaic` decode. No opcode list returns
  before any conversion or temporary allocation.
- Pointwise and separable opcodes use independent row bands. A single
  neighborhood barrier uses a two-row halo. Multiple neighborhood barriers use
  the prior full-plane algorithm as the exact fallback.
- The current f32 operation order and `roundf` behavior remain unchanged until
  an exhaustive identity proof exists.
- Pentax 645Z has no strip/tile scheduler opportunity: its one strip is the
  entropy segment. The shared kernel remains unchanged under the written
  restart-free single-scan gate.
- R5/R6/R7 have one tile, four independent planes, one subband per plane, and
  zero transform levels. Placement is already fused in the direct path.
- Phase 1 retained two independently measured commits: 32K-sample row bands
  and a fused zero-constant bad-pixel path. Final 16T times are 23.57 ms for
  G1 X, 63.52 ms for Nokia, and 50.99 ms for Leica SL2.
- Phase 2 stopped without a code change: Pentax 645Z has one strip and one
  entropy segment.
- Phase 3 stopped at the checkpoint gate. The design note is
  [CRX intra-plane start](chore-crx-intra-plane-start.md).
- Phase 4 rejected tile-first Deflate reconstruction because it regressed all
  three float rows at 16T and raised peak memory.
- Phase 5 retained Sony pair placement and zero-point-transform specialization;
  the Nikon paired packed-group candidate regressed 16T and was dropped.
