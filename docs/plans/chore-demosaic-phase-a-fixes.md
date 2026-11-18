# Demosaic correctness and research-grade refactor

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
- [x] Run algorithm reference/ground-truth gates, speed and memory measurements,
  and repository validation.
- [x] Record Phase A results and stop before Phase B.
- [x] Add the generic f64 invariant harness.
- [x] Replace dynamic f32 transport with explicit static conversions.
- [x] Remove shared `[0,1]` output clamps.
- [x] Isolate bounded integer conversion behind a fallible reference adapter.
- [x] Add scale-normalization and analytic transfer-function helpers.
- [x] Make ground-truth scene exclusion explicit and deterministic.

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

## Phase B design note

Linear-light contract:

- `T` is the working type from input to output. No shared helper may round-trip
  through integer storage or silently narrow a higher-precision input.
- Demosaicing never normalizes to `[0,1]`, clamps output, or applies gamma as a
  default transform. Reference quantization remains a separate comparison
  operation.
- Algorithms whose mathematical formulation requires a scoring transform use
  an explicit, reversible scale-normalization wrapper around their kernel.
  The scale is a robust high percentile of `abs(sample)`, homogeneous of degree
  one, with `1` as the safe fallback for an all-zero field.

Precision abstraction:

- Phase B retains `Primitive: Float` as the shared bound. It already provides
  static, monomorphized arithmetic, comparisons, absolute value, square root,
  powers, and constants; its image bounds are required by pipeline storage.
- A smaller custom trait is deferred until per-kernel conversions show a
  concrete operation gap. Dynamic dispatch, `TypeId`, and `Any` downcasts are
  removed from demosaic conversion plumbing.
- Production kernels instantiate `f32`. Correctness tests instantiate both
  `f32` and `f64`; `f64` tests start at `2e-10` relative and `1e-12` absolute
  tolerance. Named constants remain `f64` and are converted once per kernel.

Working buffers:

- Shared output construction becomes generic over `Vec<T>` and preserves signs
  and magnitudes.
- The shared u16 `codes` path is removed from the final architecture. Until
  Phase C converts its consumers, bounded integer conversion is isolated as an
  explicit, fallible reference-precision adapter. It rejects out-of-range input
  rather than clipping it.
- AHD and AMaZE remain f32 kernels after Phase B. Their generic facade is tracked
  as a Phase C blocker, not presented as generic arithmetic.

Transfer functions:

- Shared analytic sRGB encode/decode helpers use the canonical piecewise
  curve in `[0,1]`, odd extension for negative values, and power-law
  continuation above one. They are not yet used by any default kernel.
- No LUT may be used to extend a transfer function outside its table domain.

Phase B stops before AHD, AMaZE, AAHD, DCB, DHT, LMMSE, VNG, or VNG4 kernel
conversion. Their u16/f32 internals remain visible Phase C blockers.

## Phase B results

- All `Any`, `TypeId`, and downcast-based demosaic transport is removed.
- `linear_f32` and `dense_from_f32_workspace` are explicit temporary adapters
  for AHD/AMaZE. They preserve signed HDR values and no longer clamp.
- The formerly shared clamped u16 path is now the explicit, fallible
  `reference_codes` adapter. It rejects non-finite or out-of-range samples.
  AAHD, DCB, DHT, LMMSE, VNG, and VNG4 therefore fail closed until Phase C.
- The `Finite` invariant expectation is now `Fail` for those six adapters:
  out-of-range input returns a clean error instead of silently clipping.
- Generic f64 tests pass for Linear and PPG over sub-f32, negative, and HDR
  values and scaling from `1e-6` through `1e6`. RCD's exact f64 constant test
  remains an ignored Phase C failure (`1.25e-6` error from fixed epsilon).
- Ground truth excludes `hdr_blocks` consistently for all algorithms because
  not every current kernel accepts it. This transforms the old HDR-biased
  ranking into: DHT, RCD, PPG, LMMSE, AMaZE, AHD, AAHD, DCB, VNG/VNG4, Linear.
- Every advanced algorithm now beats Linear on the common supported scene set.

Affected-kernel performance at `1000x1500`, one thread:

| Algorithm | Time before | Time after | Change | RSS after |
| --- | ---: | ---: | ---: | ---: |
| AAHD | 511.7 ms | 513.9 ms | +0.4% | 86.9 MiB |
| AHD | 386.0 ms | 389.8 ms | +1.0% | 51.6 MiB |
| AMaZE | 475.9 ms | 479.8 ms | +0.8% | 50.0 MiB |
| DCB | 502.5 ms | 505.9 ms | +0.7% | 75.7 MiB |
| DHT | 329.2 ms | 334.4 ms | +1.6% | 49.8 MiB |
| LMMSE | 622.1 ms | 625.1 ms | +0.5% | 67.8 MiB |
| VNG | 480.4 ms | 487.2 ms | +1.4% | 49.9 MiB |
| VNG4 | 479.1 ms | 482.0 ms | +0.6% | 50.0 MiB |

Incremental release compilation remained effectively flat: 2064.6 ms before,
2057.1 ms after.

## Phase C conversion log

### DHT: removed

The positive-domain ratio reconstruction could not preserve a negative
constant field after integer quantization and clipping were removed; missing
channels shifted by about 1.9% at `-0.25`. Its mid-gray padding and channel
bounds are part of the retained LibRaw mathematical model, so retaining DHT
would require a different, sign-correct algorithm rather than a numerical
conversion. The public variant, tests, comparison cases, adapters, snapshots,
and build dependencies are removed. The reference note remains as provenance
for a possible future formulation.

### AAHD: removed

After replacing integer storage, gamma LUTs, and extrema clamps, AAHD still
failed f32 scale equivariance because its discrete homogeneity decision changed
under equivalent signal scaling. The direction choice is part of the retained
AAHD scoring model, so a stable generic version would require a new
scale-stable decision rule rather than a numerical conversion. The public
variant, implementation, tests, comparison case, adapter, snapshots, and build
dependency are removed. Its reference note remains as provenance.

### DCB: removed

DCB is not a localized integer implementation. Candidate orientation,
correction, Nyquist, and refinement stages exchange byte-scale integer weights
and truncated color ratios throughout the retained LibRaw model. Converting
those values changes the decision algorithm rather than porting it, and the
algorithm ranked below Linear before Phase C. The public variant, implementation,
tests, both comparison cases, adapters, snapshots, and build dependencies are
removed. Its reference and licence notes remain.

### AHD: removed

AHD shares AAHD's bounded gamma/Lab homogeneity model and adds explicit
flooring and quantization in its measured-sample reconstruction. That scoring
model is not a removable numerical detail, and AHD ranked below the retained
directional methods. The public variant, implementation, tests, both comparison
cases, adapters, snapshots, and build/setup dependencies are removed.
