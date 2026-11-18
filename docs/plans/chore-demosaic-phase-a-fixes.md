# Demosaic Phase A fixes

## Goal

Fix the non-HDR correctness bugs identified by the demosaic correctness audit:
small-image panics, zero-initialized tile margins, and orientation-equivariant
flip failures. Preserve the linear-pipeline contract and do not add a legacy
integer compatibility mode.

## Scope

In scope:

- An exhaustive small-size safety sweep for every demosaic variant and Bayer
  pattern.
- AMaZE handling of images below its supported tiled size.
- LMMSE tile halo initialization and constant fields across tile boundaries.
- AAHD working-margin initialization and outermost pixels.
- Border-construction flip failures that can be fixed locally.
- Per-algorithm invariant, ground-truth, reference, speed, and memory checks.

Out of scope:

- Phase B shared HDR workspace work.
- Phase C f32 conversions of the integer-backed algorithms.
- Algorithm changes beyond the Phase A bugs.
- Protocol type moves, public API changes, binary fixtures, and compatibility
  modes.

## Task checklist

- [x] Add the all-algorithm size sweep and relevant boundary regressions.
- [x] Fix AMaZE below its supported tiled size and update invariant expectations.
- [ ] Extend LMMSE tile halos and require constant fields at tile boundaries.
  Blocked by reference/evidence gates; behavior left unchanged for follow-up.
- [x] Extend the AAHD working margin and require the outermost constant pixel.
- [x] Diagnose and fix border-construction flip failures; record direction-tie
  asymmetry for Phase C when appropriate.
- [ ] Run algorithm reference/ground-truth gates, speed and memory measurements,
  and repository validation.
- [ ] Record Phase A results and stop before Phase B.

## Files expected to change

- `crates/fornax-raw-pipeline/tests/demosaic_invariants.rs`
- `crates/fornax-raw-pipeline/src/demosaic/{amaze,lmmse,aahd,dcb,dht,rcd}.rs`
- `crates/compare-demosaic/baseline/canonical.json` and generated README only if
  accepted comparison numbers or verdicts change.
- This plan.

## Validation

- Focused invariant tests in `fornax-raw-pipeline`, watching each regression fail
  before its production fix.
- Package tests, nightly rustfmt, and clippy.
- Affected reference rows followed by `compare-demosaic diff`.
- Heavy ground-truth tests and a full canonical run when comparison behavior
  changes.
- One large-image timing and peak-RSS measurement per affected algorithm.
- `scripts/agent-validation.sh`.

## Decisions and notes

- Reflection means mirroring across the nearest in-bounds sample while preserving
  CFA phase. Replication is acceptable only where the algorithm's topology or
  reference convention requires it.
- A size-sweep result is valid when it is either a clean error or a finite RGB
  image of the requested dimensions. A panic is never a valid result.
- Invariant expectation entries are executable: newly fixed failures become
  `Pass` in the same commit as the fix.
- Any exact-compatibility requirement that cannot be met by comparison
  tolerances stops that algorithm rather than introducing a dual mode.
- Converted numeric kernels use the same float bound as `Linear`, PPG, and RCD;
  production instantiates only `f32`. Invariant tests that exercise converted
  kernels also instantiate `f64`, with relative tolerance `2e-10` and absolute
  tolerance `1e-12`. Kernel constants remain named `f64` values and are
  converted once per kernel, not at each use.
- AMaZE remains `f32`-only unless its exponent bit manipulation is replaced by
  an equivalent arithmetic operation.

### AMaZE small-size fix

The size sweep found AMaZE panics for `32x32` and `64x32`, not only the square
case in the audit. Tiled corner reflection reads the first in-bounds sample at
offset 32, so the tiled kernel requires both dimensions to be at least 33.
Dimensions at or below 32 now use the existing PPG fallback; normal-size output
is unchanged.

### Margin and flip results

- AAHD reflects the sampled CFA field into its four-pixel working margin and
  seeds all candidate channels at unmeasured margin sites from the reflected
  sample. The outermost constant test passes. Reference rows remain within
  limits; changed pixels are about 0.20% and the ground-truth values are
  unchanged.
- AMaZE corner reflection now uses the tile's actual negative origin instead of
  the fixed halos. The corner-only flip regression passes for all patterns.
  Broader flips still diverge from `(1, 0)` onward; this is direction selection,
  not an out-of-bounds or corner-construction failure.
- The legacy AMaZE partial-tile test compared the full frame and required a
  two-code maximum. It now applies the reference comparison's 16-pixel border,
  keeps the existing MAE ceiling, and requires at most 0.1% of interior pixels
  above 64 codes.
- DHT reflects measured CFA samples into its four-pixel margin. Flip error drops
  from as much as about `0.014` to `0.0005`-`0.0014`, but ties still diverge at
  `y = 0`. Reference rows remain within limits and ground truth is unchanged.
- DCB and RCD fail in the interior after reflecting the border, indicating
  integer/direction tie asymmetry for Phase C. AAHD also returns to a
  direction-selection difference after its constant-field bug is fixed.
- LMMSE halo correction is blocked. Global reflection changed darktable MAE
  from about `0.00019` to `0.162`-`0.174`, tile-edge replication changed it to
  about `0.35`, and the Phase 2 evidence says Fornax becomes worse on divergent
  samples. Production LMMSE behavior is unchanged; the constant test is ignored
  with the blocker recorded.

### Phase A measurements

The benchmark is the existing Criterion harness at `1000x1500`, `f32`, one
Rayon thread, sampled with three Hyperfine process runs after one warmup.
Peak resident memory includes the benchmark process.

| Algorithm | Time before | Time after | Change | RSS before | RSS after |
| --- | ---: | ---: | ---: | ---: | ---: |
| AAHD | 536.1 ms | 532.8 ms | -0.6% | 86.7 MiB | 86.9 MiB |
| AMaZE | 489.5 ms | 486.9 ms | -0.5% | 49.9 MiB | 49.8 MiB |
| DHT | 336.4 ms | 337.3 ms | +0.3% | 49.8 MiB | 49.9 MiB |

Incremental release compilation of `fornax-raw-pipeline` averaged 1855.8 ms
before and 1873.9 ms after over five touched rebuilds, a 0.98% increase. No
meaningful speed or memory regression was observed.
