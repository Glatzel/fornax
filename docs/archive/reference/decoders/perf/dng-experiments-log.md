> Status: archived historical experiment log.
> Current documents: [raw-decode comparison](../../../../reference/decoders/perf/flashraw-vs-fornax.md) and [large-page allocation design](../../../../reference/decoders/design/large-page-allocation.md).
> Archived 2026-11-16; negative results are retained as provenance.

# DNG Performance Experiment Log

This log preserves the kept and reverted DNG performance experiments behind
[DNG Decode Remaining Performance Gap](dng-remaining-gap.md).

The current report is authoritative for the final state. Evidence files are in
[the DNG gap profile index](dng-gap-profiles/README.md), and reproduction steps
are in [RESUME.md](dng-gap-profiles/RESUME.md). Measurements are host-specific:
WSL2 Debian, AMD Ryzen 7 5800H, kernel
`6.6.87.1-microsoft-standard-WSL2`, glibc `2.41-12+deb13u3`, THP `[madvise]`,
and THP defrag `[madvise]`.

## Decode and allocation experiments

| Experiment | Tried at | Key measured result | Decision |
| --- | --- | --- | --- |
| Buffered LJPEG error state | PR #724, commits `fc668a4` and `b4222de` | 5D3 LJPEG 267.899 -> 203.723 ms; Pixel 8 559.753 -> 418.696 ms | Kept |
| Compact 10-bit Huffman table with branchless amplitude/sign extension | PR #737, commit `f06cb2e` | 5D3 1T 175.302 -> 157.404 ms; Pixel 8 1T 360.765 -> 331.464 ms; L1D misses 0.96167 -> 0.06010/sample on 5D3 | Kept |
| Natural-alignment zeroed allocation plus Linux `MADV_HUGEPAGE` | PR #738, commit `5ddd07e`; removal was PR #734, commit `d455910` | 5D3 uncompressed 47.357 -> 18.400 ms; FP32 Deflate 401.427 -> 340.140 ms; 5D3 LJPEG 177.101 -> 143.696 ms | Kept |
| Register-resident LJPEG reader locals | Branch `chore/ljpeg-instruction-gap`, commit `8007095`; reverted by `9a85782` | 5D3 instructions 57.329 -> 48.126/sample, but cycles 23.981 -> 24.809 and wall 179.603 -> 184.238 ms | Reverted |
| Unconditional LJPEG refill | Branch `chore/ljpeg-instruction-gap`, commit `3ae7a9b`; reverted by `0c35bce` | Branch misses 0.10636 -> 0.01814/sample, but cycles 20.702 -> 23.330 and wall 162.439 -> 178.382 ms on 5D3 | Reverted |
| 12-bit fused Huffman table | PR #734 portable experiments | Two-component noisy synthetic scan +21%; no consistent real-file win | Reverted |
| 11-bit fused Huffman table | PR #734 portable experiments | Two-component noisy synthetic scan +23%; no consistent real-file win | Reverted |
| Canonical long-code path without the 16-bit table | PR #734 portable experiments | 5D3 +1.2% at 1T and +8.6% at 15T; Pixel 1T -5.7%; instructions 100.66 -> 105.22/sample on 5D3 | Reverted |
| Shared `Arc` Huffman tables | PR #734 portable experiments | 5D3 +21% and +15.7%; Pixel 1T -1.2% and 15T +4.9% | Reverted |
| Fused const-generic direct-band kernel | Branch `chore/portable-ljpeg-fused-kernel`, commit `33fccea` | 5D3 +1.6%, iPhone SE +11.4%, Pixel 8 -3.7% at 1T; no stable all-thread win | Reverted |
| Unstuff once into reusable scratch | Branches `chore/dng-followup-probes` and `chore/ljpeg-fault-followup` | Smooth 64-tile scans improved 0.9-19.1%, but noisy scans regressed 1.7-5.7%; faults were effectively unchanged | Reverted |
| Component-pair refill | PR #739 follow-up | 5D3 1T 130.619 -> 144.475 ms; instructions 79.580 -> 98.183/sample | Reverted |
| Fixed 12-bit packed-`u32` full-difference table | PR #739 follow-up | Tiled LJPEG improved 3.4-11.9%, but Pentax K-5 regressed 14.8-16.0%; cycles 22.134 -> 25.819/sample and L1D misses 0.0872 -> 0.1583/sample on Pentax 1T | Reverted |
| Per-worker Huffman-buffer reuse | PR #730 | 5D3 faults 30,683 -> 3,033, but wall time was neutral on 5D3 and Pixel 8 15T regressed 1.93% | Reverted |
| `zlib-rs` Deflate backend | PR #734 portable experiments | FP16 1T -5.8% and 15T +1.0%; FP32 1T -10.5% and 15T -5.1% | Reverted |
| Caller-owned LJPEG output prototype | PR #734, commit `4d38e42` | Synthetic reuse saved 1.6-5.4%; 5D3-sized full-output reuse saved 1.6% | Reverted; public API proposal stopped |
| Fat LTO | PR #734 portable experiments | 5D3 +6.5/+9.6%; Pixel 8 +0.7/+8.0% at 1T/15T | Reverted |
| Restart-free single-scan parallelism | PR #739 design gate | No implementation; exact boundary discovery would duplicate the hot Huffman walk | Stopped |

The caller-owned-output proposal is an additive `decode_into` API, not an
accepted change. It remains blocked on an explicit public-API decision and
partial-write semantics.

## Experiment branches and worktrees

These branches and worktrees preserve negative results or reproduction code.
Keep all of them for now; cleanup must not depend on deleting them.

| Branch or detached worktree | Worktree | Recommendation |
| --- | --- | --- |
| `task/perf-dng-decode` | `.worktrees/perf-dng-decode` (prunable metadata) | Keep |
| `chore/close-dng-flashraw-gap` | `.worktrees/chore-close-dng-flashraw-gap` | Keep |
| `chore/dng-first-touch-probe` | `.worktrees/chore-dng-first-touch-probe` | Keep |
| `chore/dng-followup-probes` | `.worktrees/chore-dng-followup-probes` | Keep |
| `chore/large-page-allocation` | `.worktrees/chore-large-page-allocation` | Keep |
| detached `985c704` baseline | `.worktrees/chore-large-page-allocation-baseline` | Keep |
| `chore/ljpeg-critical-path` | `.worktrees/chore-ljpeg-critical-path` | Keep |
| `chore/ljpeg-fault-followup` | `.worktrees/chore-ljpeg-fault-followup` | Keep |
| `chore/ljpeg-instruction-gap` | `.worktrees/chore-ljpeg-instruction-gap` | Keep |
| `chore/portable-ljpeg-fused-kernel` | `.worktrees/chore-portable-ljpeg-fused-kernel` | Keep |
| `chore/portable-scalar-codec` | `.worktrees/chore-portable-scalar-codec` | Keep |
| `chore/flashraw-cpu-comparison` | `.worktrees/chore-flashraw-cpu-comparison` | Keep |
| `chore/flashraw-cpu-followup` | `.worktrees/chore-flashraw-cpu-followup` | Keep |
| `chore/flashraw-cpu-source` | none listed | Keep |
| `chore/arw-gap-profiles` | `.worktrees/chore-arw-gap-profiles` | Keep |
| detached LJPEG baseline | `/tmp/fornax-ljpeg-baseline` (prunable metadata) | Keep |
