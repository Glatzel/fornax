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

### Shared transport cleanup

With AHD removed, no retained kernel needs the temporary f32-only input or
output adapters introduced during Phase B. The dead helpers and their focused
tests are removed; dense output construction now stays generic over `T`.

### Comparison test policy

The native VNG, VNG4, and LMMSE tests that required pass limits against the
bounded integer references are removed. Reference agreement is now judged by
the canonical status rule, which combines the disagreement floor with
ground-truth evidence; a kernel is not rejected merely for differing from a
quantized reference. The RawTherapee adapter test now enumerates the IDs that
remain registered after AHD removal instead of probing the removed ID.

### Preprocessor sub-black preservation

Black subtraction previously floored both fast and per-row black-delta paths at
zero. The normalization now preserves signed sub-black samples in both paths
while retaining zero output for a non-positive black/white range, where
division is undefined. A focused f64 regression covers both paths. All pipeline
tests pass; no consumer test or public handoff depended on the former floor.

## Phase C results

### Retained implementation

The retained variants are Linear, PPG, RCD, AMaZE, VNG, VNG4, and LMMSE.
Each kernel is generic over the shared `Primitive: Float` bound and production
instantiates `f32`. The trait implementations are only `f32` and `f64`; integer
types do not satisfy it. DHT, AAHD, DCB, and AHD were removed because their
mathematical direction or scoring models depended on bounded code values,
integer quantization, or gamma-domain decisions rather than a removable output
clamp. Their source notes remain under `docs/reference/algorithms/` as
provenance for possible future formulations.
The public CLI now rejects the four removed names and accepts only registered
variant IDs.

AMaZE's exponent bit manipulation was replaced by floating-point powers of two,
so it is generic rather than `f32`-only. LMMSE uses the shared analytic transfer
function with odd extension for negatives and power-law continuation above one.
No retained production path contains a u16 working buffer, output clamp, gamma
lookup table, or implicit conversion to a narrower working type.

### Invariant matrix

`Pass` means all four Bayer patterns pass at the executable f32 tolerance.
Converted kernels also have focused f64 scale, signed-range, and constant-field
tests at `2e-10` relative and `1e-12` absolute tolerance.

| Algorithm | Scale | Shift | Flip | Constant | Ramp | Tiling | Finite | Negative | HDR |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Linear | Pass | Pass | Pass | Pass | Pass | Pass | Pass | Pass | Pass |
| PPG | Pass | N/A | Pass | Pass | N/A | Pass | Pass | Pass | Pass |
| RCD | Pass | N/A | Fail | Pass | N/A | Pass | Pass | Pass | Pass |
| AMaZE | Pass | N/A | Fail | Pass | N/A | Pass | Pass | Pass | Pass |
| VNG | Pass | N/A | Pass | Pass | Pass | Pass | Pass | Pass | Pass |
| VNG4 | Pass | N/A | Pass | Pass | N/A | Pass | Pass | Pass | Pass |
| LMMSE | Pass | N/A | Pass | Pass | N/A | Pass | Pass | Pass | Pass |

Shift is not applicable to non-additive methods. Ramp claims are limited to the
exact Linear and VNG cases. The remaining flip failures are deterministic:
AMaZE first differs at `(1, 0)` by `0.006788` for RGGB to GRBG; RCD first
differs at `(9, 9)` by `0.028822`. They are direction/tie asymmetry, not the
Phase A crash or corner-reflection defects.

### Ground-truth ranking

With all retained kernels now accepting HDR input, `hdr_blocks` is included and
the excluded-source list is empty. On the full deterministic synthetic and
corpus set:

| Rank | Algorithm | Relative mean | vs Linear | Suspicious scenes |
| ---: | --- | ---: | ---: | --- |
| 1 | RCD | 0.065768 | 0.889x | none |
| 2 | PPG | 0.068210 | 0.922x | colored_edges |
| 3 | LMMSE | 0.069145 | 0.935x | colored_edges |
| 4 | AMaZE | 0.069376 | 0.938x | colored_edges |
| 5 | VNG | 0.073676 | 0.996x | slanted_edge, colored_edges |
| 6 | VNG4 | 0.073676 | 0.996x | slanted_edge, colored_edges |
| 7 | Linear | 0.073991 | 1.000x | none |

This ordering is consistent with the literature expectation used by the
harness. The scene sets are not directly comparable to Phase B because HDR is
now admitted. The recorded supported-scene comparisons show AMaZE moving from
`0.075552` to `0.075941` and VNG/VNG4 from `0.078106` to `0.078452`, increases
of 0.52% and 0.44%. These exceed a strict zero-regression gate even though they
remain below the 1% suspicious-source margin and the mathematical invariants
improve.

### Reference verdicts

The native RGGB rows below report code-domain MAE/P99/max. `⚖️` uses synthetic
ground-truth error over pixels beyond the reference tolerance; it is accepted
only when Fornax MAE and P99 are no more than one code worse than the reference.

| Algorithm/reference | Before | After | Current evidence |
| --- | --- | --- | --- |
| AMaZE / darktable and RawTherapee | ❌, MAE `0.003868`, P99 `0`, max `321` | ⚖️, MAE `0.678612`, P99 `13`, max `2252` | Fornax MAE/P99 `1150/6430`; reference `1352/6803` |
| LMMSE / darktable | ✅, MAE `0.000189`, P99 `0`, max `1` | ❌, MAE `0.811433`, P99 `17`, max `516` | Fornax `3711/44276`; reference `3619/44473` |
| LMMSE / RawTherapee | ❌, MAE `4.621489`, P99 `40`, max `3028` | ❌, MAE `4.395130`, P99 `37`, max `3028` | Fornax `3593/46322`; reference `3506/45991` |
| RCD / darktable | ✅, MAE `0.176026`, P99 `4`, max `277` | ❌, MAE `0.057511`, P99 `1`, max `6` | Fornax `2006/4916`; reference `1951/4699` |
| RCD / RawTherapee | ❌, MAE `0.342880`, P99 `6`, max `1902` | ❌, MAE `0.224195`, P99 `1`, max `1902` | Fornax MAE `4115` is better than `7788`, but P99 `50110` is worse than `27275` |
| VNG / darktable | ❌, MAE `0.380463`, P99 `1`, max `894` | ❌, MAE `9.545903`, P99 `63`, max `3239` | Fornax `1421/11227`; reference `1310/9691` |
| VNG / LibRaw | ❌, MAE `17.639879`, P99 `117`, max `2958` | ❌, MAE `21.958831`, P99 `138`, max `3234` | Fornax `1986/17726`; reference `1261/11891` |
| VNG4 / LibRaw | ✅, MAE `0`, P99 `0`, max `0` | ❌, MAE `9.614591`, P99 `63`, max `3240` | Fornax `1424/11227`; reference `1311/9692` |
| VNG4 / RawTherapee | ⚖️, MAE `9.561479`, P99 `75`, max `3233` | ⚖️, MAE `15.916777`, P99 `103`, max `3739` | Fornax MAE/P99 `2232/27272`; reference `2812/35729` |

PPG and Linear verdicts are unchanged. Generated README notes now describe the
linear-light implementations instead of the former clamps and integer paths.

### Performance and memory

Measured on the existing Criterion `1000x1500`, f32, one-thread case with three
Hyperfine process runs after one warmup. Peak RSS is the benchmark process.

| Algorithm | Main time | Current time | Time change | Main RSS | Current RSS | RSS change |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Linear | 171.3 ms | 162.6 ms | -5.1% | 33.1 MiB | 32.9 MiB | -0.6% |
| PPG | 272.0 ms | 262.1 ms | -3.6% | 32.9 MiB | 32.9 MiB | 0.0% |
| RCD | 311.0 ms | 312.9 ms | +0.6% | 33.5 MiB | 39.5 MiB | +17.9% |
| AMaZE | 487.6 ms | 481.8 ms | -1.2% | 52.5 MiB | 52.4 MiB | -0.2% |
| VNG | 495.0 ms | 582.5 ms | +17.7% | 53.2 MiB | 56.0 MiB | +5.3% |
| VNG4 | 481.9 ms | 587.1 ms | +21.8% | 53.3 MiB | 55.9 MiB | +4.9% |
| LMMSE | 635.5 ms | 668.8 ms | +5.2% | 68.4 MiB | 56.5 MiB | -17.4% |

No slowdown exceeds the 25% review threshold and no memory growth approaches
2x. Five touched incremental release builds averaged `0.868 s` on this branch
versus `1.860 s` on main, so retained-code compile time did not regress.

## Phase D final assessment

### Task checklist

- [x] Expand AMaZE synthetic evidence across seeds, scene variants, and sizes.
- [x] Run f64 scale and constant-range checks across every retained algorithm,
  Bayer pattern, two additional dimensions, and two seeds.
- [x] Re-triage all eleven algorithms with invariants, ground truth, and
  reference status.
- [x] Regenerate canonical, snapshots, and README from the full comparison run.
- [x] Update `TODO.md` with remaining demosaic work only.

### AMaZE evidence expansion

AMaZE reference evidence now uses 24 deterministic sources. The original six
ground-truth scenes remain first, followed by three variants of each scene at
`96x80`, `126x94`, and `160x128`, with distinct seeds and altered geometry or
texture. Other algorithms keep the original six-source evidence set so the full
canonical run does not multiply every reference row.

| Pattern | Reference | Divergent samples | Fornax MAE / P99 | Reference MAE / P99 | Status |
| --- | --- | ---: | ---: | ---: | --- |
| BGGR | darktable / RawTherapee | 68,324 | 14,900.19 / 52,178 | 14,934.32 / 52,561 | ⚖️ |
| GBRG | darktable / RawTherapee | 67,553 | 15,160.93 / 52,178 | 15,183.72 / 52,561 | ⚖️ |
| GRBG | darktable / RawTherapee | 66,600 | 14,812.04 / 52,179 | 14,865.81 / 52,562 | ⚖️ |
| RGGB | darktable / RawTherapee | 68,193 | 14,850.46 / 52,178 | 14,916.98 / 52,561 | ⚖️ |

All eight AMaZE rows remain accepted, with the same result from both references.
The earlier sparse three-pattern result was fixture-limited; the current
linear-light AMaZE has a stable evidence advantage rather than a narrow
geometry-specific pass.

### Combined triage

`Pass` means every applicable f32 invariant passes. Converted kernels also pass
the expanded f64 scale and signed/HDR constant checks.

| Algorithm | Invariants | GT rank / vs Linear | Reference status | Classification / disposition |
| --- | --- | --- | --- | --- |
| RCD | Flip fail; other applicable checks pass | 1 / 0.889x | darktable ❌; RawTherapee ❌ | Direction-tie asymmetry plus marginal evidence-worse tails. Retained because it is the best ground-truth method. |
| PPG | All applicable checks pass | 2 / 0.922x | darktable ✅; LibRaw ✅ | Retained, linear-light floor above Linear. |
| LMMSE | All applicable checks pass | 3 / 0.935x | darktable ❌; RawTherapee ❌ | Analytic linear-light formulation differs from bounded gamma references and is evidence-worse on divergent samples. Retained on invariant and GT quality. |
| AMaZE | Flip fail; other applicable checks pass | 4 / 0.938x | both references ⚖️ | Direction-tie asymmetry remains. Broad evidence supports retained linear-light behavior. |
| VNG | All applicable checks pass | 5 / 0.996x | darktable ❌; LibRaw ❌ | Evidence-worse against both references despite passing the LibRaw disagreement floor. Retained but open. |
| VNG4 | All applicable checks pass | 6 / 0.996x | LibRaw ❌; RawTherapee ⚖️ | Shares VNG evidence and performance limitations. Retained but open. |
| Linear | All applicable checks pass | 7 / 1.000x | LibRaw 🟢 | Retained correctness floor. |
| AHD | Removed: HDR/negative/scale failures | Removed | Removed | Bounded gamma/Lab homogeneity scoring and quantization are central to the retained formulation. |
| AAHD | Removed: constant/HDR/negative/scale failures | Removed | Removed | Gamma/LUT scoring and discrete direction choices were not convertible without changing the algorithm. |
| DCB | Removed: HDR/negative/scale failures | Removed | Removed | Byte-scale candidate/refinement decisions and truncating ratios are pervasive. |
| DHT | Removed: HDR/negative/scale failures | Removed | Removed | Mid-gray padding, code thresholds, and bounded reconstruction are part of the retained model. |

### Remaining work

| Priority | Open item | Evidence | Likely cause | Effort | Gain |
| ---: | --- | --- | --- | --- | --- |
| 1 | AMaZE/RCD flip symmetry | AMaZE differs at `(1,0)` by `0.006788`; RCD at `(9,9)` by `0.028822` | AMaZE tile/halo and tie selection (`amaze.rs:376`, `amaze.rs:815`, `amaze.rs:1618`); RCD tile-relative direction weights (`rcd.rs:141`, `rcd.rs:301`, `rcd.rs:380`) | Medium | Two invariant failures become Pass; improves orientation reliability. |
| 2 | LMMSE reference evidence | darktable evidence MAE `3711` vs `3619`; RawTherapee `3593` vs `3506` | Analytic transfer and corrected signed/HDR branch (`lmmse.rs:36-92`) intentionally differs from bounded reference tables | Medium-high | Could turn both reference rows from ❌ to evidence-supported status without reintroducing clamps. |
| 3 | VNG/VNG4 reference evidence and GT quality | Both rank `0.996x` vs Linear; evidence MAE/P99 worse than reference | Float gradient selection and corrected interpolation replace truncating integer decisions (`vng.rs:136`, `vng.rs:160`, `vng.rs:254`) | High | Largest remaining quality gain among retained methods. |
| 4 | AMaZE/VNG supported-scene regression | AMaZE `0.075552` to `0.075941`; VNG/VNG4 `0.078106` to `0.078452` | AMaZE highlight/local limits and VNG direction changes after bounded arithmetic removal | Medium | Restores a strict no-regression gate while retaining invariants. |
| 5 | VNG/VNG4 speed | About 18-22% slower than main f32 benchmarks | Generic phase samples and repeated `powi` gradient weights (`vng.rs:97`, `vng.rs:160`) | Medium | Removes the only retained-kernel slowdown above 15%. |
| 6 | RCD reference evidence tail | DNG MAE improved to `0.0575`/`0.2242`, but evidence P99 remains worse than references | Sub-code ratio/tie interpretation propagates through discrimination weights (`rcd.rs:222`, `rcd.rs:301`, `rcd.rs:380`) | Medium | Could clear reference ❌ without changing the best aggregate GT result. |
| 7 | Future linear reformulations of removed algorithms | AHD/AAHD/DCB/DHT cannot meet the current linear-light architecture | Their mathematical direction models depend on bounded, gamma, or integer code behavior | High | Only justified as new algorithms, not compatibility wrappers. |
