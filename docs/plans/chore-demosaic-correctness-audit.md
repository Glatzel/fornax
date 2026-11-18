# Demosaic correctness audit

## Goal

Judge Fornax demosaic output with three independent checks:

1. HDR-safe algebraic and structural invariants.
2. Reference-independent ground-truth quality.
3. Principled agreement with integer references where agreement is meaningful.

Reference fidelity remains useful evidence, but it is not the sole definition of
correctness. Demosaicing is a linear-pipeline stage: it must not clamp RGB to
`[0, 1]`. Display and export own that clamp.

## Scope

In scope:

- All eleven `Demosaic` variants: `Linear`, `PPG`, `RCD`, `AHD`, `AAHD`,
  `AMaZE`, `DCB`, `DHT`, `LMMSE`, `VNG`, and `VNG4`.
- Shared demosaic conversion and output helpers.
- Fast seeded invariant tests in `fornax-raw-pipeline`.
- A reference-independent ground-truth harness in `compare-demosaic`.
- Valid-range reference comparison, reference disagreement floors, distribution
  metrics, and evidence-based `⚖️` status.
- A final diagnosis and ranked fix list for the six algorithms named by the
  request.

Out of scope:

- Changing demosaic algorithm behavior in Phases 0-3.
- Fixing any algorithm in Phase 4.
- Moving demosaic protocol types between crates.
- Non-Bayer CFA support.
- Committing RAW files, generated images, or other binary test data.

## Task checklist

- [x] Phase 0: audit range assumptions, CFA assumptions, and invariance classes.
- [ ] Phase 1: add fast seeded invariant tests and record expected failures.
- [ ] Phase 2: add deterministic synthetic and optional corpus-binned ground
  truth, HDR-safe metrics, ranking, schema versioning, and README output.
- [ ] Phase 3: make reference comparison valid-range aware, mask borders, report
  disagreement floors and distributions, and derive evidenced `⚖️`.
- [ ] Phase 4: re-triage AAHD, AMaZE, DCB, DHT, LMMSE, and VNG; report root
  causes and propose fixes without implementing them.

## Files expected to change

- This plan, updated after each phase with its short report.
- Phase 1: `fornax-raw-pipeline` demosaic test modules only.
- Phase 2: `compare-demosaic` run/report/metrics/readme code,
  `baseline/canonical.json`, and generated README regions.
- Phase 3: `compare-demosaic` run/report/status/readme code, baseline, and
  generated README regions.
- Phase 4: this plan only.

## Validation

- Phase 0: `scripts/agent-validation.sh --docs-only`; run the existing
  `compare-demosaic` README/schema tests and the existing fast demosaic tests.
- Phase 1: targeted `cargo test -p fornax-raw-pipeline`, then normal workspace
  validation for the changed crate.
- Phase 2 and 3: crate tests, `compare-demosaic` schema/readme checks, a full
  baseline run where required, and repository validation.
- Phase 4: documentation-only validation.

Markdown lint remains CI-only. Every phase must inspect the diff and report
failed or skipped checks exactly.

## Decisions and notes

- Use positive scale factors `k in {1e-3, 1e-1, 1, 1e1, 1e3}`. Negative
  scaling is not part of the stated scale test and would interact with
  orientation semantics.
- Treat the requested GBRG pattern as the intended canonical native pattern.
  The checked-in baseline currently records native `RGGB`; Phase 0 records the
  discrepancy without changing the baseline. Pattern tests will cover all four
  conventional Bayer patterns.
- Keep the existing three `compare-demosaic` subcommands and all existing flags.
  Prefer adding the ground-truth stage to `run` and extending canonical JSON,
  rather than introducing a fourth framework or subcommand.
- A fixed seed and deterministic scene generator are mandatory. A tiny local
  seeded generator is sufficient; no random-data framework is needed.
- Known invariant failures stay visible as ignored tests with tracking reasons
  or a checked expected-failure table. Tests will not be weakened to match the
  current implementation.

## Phase 0 audit

### Policy

Demosaic input may be negative or greater than one. Its output may also be
negative or greater than one. Clamping to `[0, 1]`, `[0, 65535]`, or nonnegative
zero is a policy violation in production demosaic code.

The audit distinguishes:

- **Fixed-range clamp:** forces pipeline values into a display/code range.
- **Data-dependent limit:** bounds an interpolation to neighboring samples.
  These are often intentional reference-compatibility behavior, but they still
  prevent arbitrary signed/HDR excursions and must be re-evaluated under the new
  contract.
- **Quantization:** conversion through `u16` itself destroys scale and HDR
  information even where a later min/max is absent.

There is no standalone JSON Schema document. The canonical schema is the strict
Serde model in `crates/compare-demosaic/src/report.rs`: unknown fields are
denied and `Report::validate` enforces shape, row ordering, metrics, and verdict
consistency. The current baseline has 76 rows, all four Bayer patterns, and
references LibRaw 0.22.3, darktable 5.6.2, and RawTherapee 5.13.

### Algorithm classification

| Algorithm | Scale invariance | Shift invariance | Evidence and primary limitation |
| --- | --- | --- | --- |
| Linear | Yes, by construction | Yes, by construction | Generic `T` uses only additions, multiplication, and division; no fixed epsilon or clamp (`linear.rs:93`, `linear.rs:108`, `linear.rs:202`). Border fallbacks are means of matching neighbors. |
| PPG | Yes for positive scale, by construction | No | Arithmetic is homogeneous and has no absolute epsilon, so fixed positive `k` scales candidate choices and output. Zero floors at `ppg.rs:111`, `ppg.rs:142`, `ppg.rs:171`, and `ppg.rs:196` break translation. |
| AAHD | Neither | No | Input is quantized after `[0,1]` clamping by `codes`, scoring uses a 16-bit gamma LUT (`aahd.rs:25`), and interpolated values are clipped to per-channel extrema (`aahd.rs:299`, `aahd.rs:329`, `aahd.rs:392`, `aahd.rs:627`). |
| AHD | Neither | No | Converts normalized values to 16-bit code scale (`ahd.rs:74`), scores in LUT-based Lab (`ahd.rs:28`, `ahd.rs:316`), clamps code values and CBRT indices (`ahd.rs:181`, `ahd.rs:323`, `ahd.rs:351`), then clamps final output. |
| AMaZE | Neither | No | Uses fixed epsilons (`amaze.rs:27`), an absolute highlight threshold of `0.8` (`amaze.rs:507`), ratio decisions, local limits, and a final `[0,1]` clamp (`amaze.rs:1547`, `amaze.rs:1564`). |
| DCB | Neither | No | Input is clamped and quantized by `codes` (`dcb.rs:47`), intermediate/final values use truncating `u16` clip helpers (`dcb.rs:321`, `dcb.rs:323`), and decisions/refinement compare integer ranges and ratios (`dcb.rs:350`, `dcb.rs:535`). |
| DHT | Neither | No | Input is clamped and quantized by `codes` (`dht.rs:717`), code padding starts at `0.5` (`dht.rs:61`), decisions use fixed code/ratio thresholds (`dht.rs:18`), and output is clamped/truncated (`dht.rs:36`). |
| LMMSE | Neither | No | Input and seed are clamped and quantized (`lmmse.rs:55`, `lmmse.rs:68`), interpolation runs in a 16-bit gamma LUT domain (`lmmse.rs:663`), and multiple intermediate/final stages clamp to unit ranges (`lmmse.rs:300`, `lmmse.rs:656`). |
| RCD | Neither | No | Ratio-corrected path uses fixed epsilons (`rcd.rs:132`), quantizes its discrimination weight to `[0,1]` (`rcd.rs:305`, `rcd.rs:411`), and floors input and output at zero (`rcd.rs:164`, `rcd.rs:503`). |
| VNG | Neither | No | Input is clamped and quantized by `codes` (`vng.rs:126`), interpolation is fixed-point `u16`/`i64`, gradient shifts and integer thresholds are used (`vng.rs:149`), and every corrected sample is clipped to a code (`vng.rs:185`, `vng.rs:316`). |
| VNG4 | Neither | No | Shares the same integer split-green VNG kernel and clips as `VNG`; only final green packing differs. |

`Linear` is currently the only variant that satisfies both the no-clamp policy
and exact algebraic scale/shift invariance. `PPG` is the only other scale-
homogeneous production path, but its zero floors make it non-shift-invariant.

### Range-sensitive and fixed-point inventory

| Algorithm | Clamps, floors, or saturation | Absolute thresholds/constants | LUT, gamma, or Lab | Integer/fixed-point state |
| --- | --- | --- | --- | --- |
| Linear | None | None | None | None; operations remain in generic `T`. |
| PPG | `.max(0)` at `ppg.rs:111`, `142`, `171`, `196`; candidate min/max limits at `108-111` | None | None | None; operations remain in generic `T`. |
| AAHD | Input `[0,1]` clamp through `buffer.rs:38`; channel-extrema `clip` at `aahd.rs:299`, `329`, `392`, implemented at `627-635` | Gamma knee `0.0181` and 16-bit coefficients at `aahd.rs:25-33` | 65,536-entry 16-bit sRGB-to-linear LUT; Rec. 2020 YUV scoring (`aahd.rs:398-415`) | `u16` image planes, `i32` gradients/YUV, `i8` flags |
| AHD | Input clamp for non-borrowed inputs (`protocol.rs:99`); neighbor clamps (`ahd.rs:142`, `152`); code clamps (`181`, `189`, `208`, `351`); LUT-index clamps (`323-325`); output clamp (`protocol.rs:134`) | CBRT knee `0.008856`; Lab scale factors and `i16` narrowing (`ahd.rs:30-34`, `327-329`) | 65,536-entry CBRT/Lab LUT | Lab stored as `i32`, narrowed through `i16` |
| AMaZE | Local `ulim` clamps (`amaze.rs:697-699`), diagonal/green neighbor clamps (`1229-1230`, `1374-1380`), final clamp/Nan fill (`1547-1564`) | `EPS = 1e-5`, `EPS_SQUARED = 1e-10` (`27-28`); highlight `0.8` (`507`); unit thresholds at `644` and `1404` | None | `f32` with bit-level exponent multiply/divide (`679-693`) |
| DCB | Input `[0,1]` clamp through `buffer.rs:38`; truncating clip calls at `dcb.rs:134`, `158`, `179`, `183`, `206`, `230`, `251`, `258`, `332`, `417`, `446`, `466`, `473`, `510-511`, `589`, `750-751`; neighbor limits at `620` and `774`; helper at `801-805` | Fixed 16-bit range; ratio correction only above one code (`dcb.rs:535`) | None | `u16` working pixels, `i32`/`f32` candidates, truncating casts |
| DHT | Input `[0,1]` clamp through `buffer.rs:38`; channel-range clamps in green/red/blue reconstruction (`dht.rs:425-434`, plus neighboring blocks); final `[0,65535]` clamp (`36`) | Hot threshold `64`, strength `256`, diagonal threshold `1.4` (`18-20`); `0.5` code pad (`61`, `80`); ratios `1.2` (`420-421`) | None | `f32` over quantized `u16` codes; direction state in `u8` |
| LMMSE | Input clamp/quantization (`lmmse.rs:55`, `68`); directional clamp `[0,1]` (`300`, `311`); negative clamp `[-1,0]` (`327`, `334`); output clamp (`656`); gamma lookup caps (`687-694`) | Gamma knees `0.001867`, `0.031746`; curve constants (`663-681`); highlight ratio `1.75` (`297`, `308`) | 65,536-entry forward and inverse gamma LUTs | `u16` codes and `f32` tile workspace |
| RCD | Zero floor for CFA input (`rcd.rs:164`), unit clamp for discrimination (`305`, `411`, `438-439`), zero floor for output (`503`) | `eps = 1e-5`, `epssq = 1e-10` (`132-133`) | None | Generic `T`, but fixed `T::one`/`T::zero` range assumptions |
| VNG/VNG4 | Input `[0,1]` clamp through `buffer.rs:38`; corrected values clipped to `[0,65535]` (`vng.rs:185`, `316`); integer division truncation (`vng.rs:183`, `246`) | Shifted gradient weights (`vng.rs:153`); half-maximum selection (`164`) | None | `u16` channels, `u32` linear sums, `i64` gradients |

### Shared and surrounding violations

| Location | Violation |
| --- | --- |
| `crates/fornax-raw-pipeline/src/demosaic/buffer.rs:38` | `codes` clamps every input to `[0,1]` and quantizes to `u16`. Used by AAHD, DCB, DHT, LMMSE, VNG, and VNG4. |
| `crates/fornax-raw-pipeline/src/demosaic/buffer.rs:44-55` | Converts `u16` output back to `[0,1]`; all integer-backed algorithms are bounded by construction. |
| `crates/fornax-raw-pipeline/src/demosaic/protocol.rs:99` | `normalized_f32` clamps converted non-borrowed inputs to `[0,1]`. Borrowed `Vec<f32>` bypasses this input clamp. |
| `crates/fornax-raw-pipeline/src/demosaic/protocol.rs:128-134` | `dense_from_f32_owned` clamps all AHD and AMaZE output, including the AMaZE small-image PPG fallback, to `[0,1]`. |
| `crates/fornax-raw-pipeline/src/demosaic/amaze.rs:1560-1564` | Replaces NaN with `0.5` and clamps every AMaZE sample to `[0,1]`. |
| `crates/fornax-raw-pipeline/src/demosaic/ppg.rs:111`, `142`, `171`, `196` | PPG forces reconstructed channels and border averages nonnegative. |
| `crates/fornax-raw-pipeline/src/demosaic/rcd.rs:164`, `503` | RCD forces input and output samples nonnegative; `305`/`411` also bound its interpolation selector to `[0,1]`. |
| `crates/fornax-raw-pipeline/src/demosaic/ahd.rs:142`, `152`, `181`, `189`, `208`, `323-325`, `351-354` | AHD actively limits candidates and output to neighbor/code ranges before the final shared `[0,1]` clamp. |
| `crates/fornax-raw-pipeline/src/demosaic/lmmse.rs:68`, `300`, `311`, `327`, `334`, `656`, `687-694` | LMMSE bounds seed, directional differences, gamma lookup, and final output to unit-scale ranges. |
| `crates/compare-demosaic/src/cases.rs:449-457` | The reference harness also clamps Fornax output to `[0,1]` before converting to 16-bit codes. This does not alter the production algorithm, but it currently makes reference comparison blind to HDR violations. |
| `crates/fornax-raw-pipeline/src/preprocessor.rs:91-92`, `107-108` | Outside the demosaic module, black/white normalization floors negative values at zero. Direct synthetic demosaic tests can exercise negative inputs, but the standard preprocessed pipeline currently cannot deliver them. |

### CFA assumptions

All production variants take `BayerPattern` and dispatch through
`BayerPattern::channel_at`; none has a production-path hard-code for RGGB or
GBRG. They support only the four conventional 2x2 RGB Bayer patterns declared in
`crates/fornax-raw-core/src/bayer.rs:31-39`.

The algorithms still assume a conventional three-color Bayer topology:

- AAHD, AHD, AMaZE, DCB, DHT, LMMSE, PPG, and RCD collapse both green phases to
  one green plane.
- VNG and VNG4 retain `GRedRow` and `GBlueRow` as separate planes until final RGB
  packing (`vng.rs:307-313`).
- Several equations use the conventional index identities `R=0`, `G=1`, `B=2`
  and `opposite = 2 - channel`; this is valid for these four patterns but does
  not generalize to X-Trans or arbitrary CFAs.

The request identifies GBRG as the canonical native pattern. The checked-in
baseline currently records `RGGB` in
`crates/compare-demosaic/baseline/canonical.json`; the existing baseline remains
fully valid and is unchanged in Phase 0.

### Phase 0 conclusion

Do not change algorithm behavior in this phase. The invariant suite in Phase 1
should intentionally expose failures for:

- input scale below `1e-5`, where fixed epsilons and `u16` quantization dominate;
- inputs above one, where integer conversion and output clamps saturate;
- negative inputs, where zero floors and code clamps remove sign information;
- translation for every variant except `Linear`;
- exact CFA symmetry where integer rounding, tile reuse, or green-phase
  conventions differ.

These are findings, not reasons to weaken the tests. `Linear` is expected to pass
all algebraic and range checks. `PPG` is expected to pass positive scale
equivariance but fail shift invariance and negative preservation.

### Phase 0 validation

- `cargo test -p fornax-raw-pipeline --lib demosaic`: 78 passed.
- `cargo test -p compare-demosaic`: 147 library tests and 3 binary tests
  passed, including strict canonical parsing and committed README checks.
- `scripts/agent-validation.sh --docs-only`: passed `git-diff` and
  `documentation-links`.
