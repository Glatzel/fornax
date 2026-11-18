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
- [x] Phase 1: add fast seeded invariant tests and record expected failures.
- [x] Phase 2: add deterministic synthetic and optional corpus-binned ground
  truth, HDR-safe metrics, ranking, schema versioning, and README output.
- [ ] Phase 3: make reference comparison valid-range aware, mask borders, report
  disagreement floors and distributions, and derive evidenced `⚖️`.
- [ ] Phase 4: re-triage AAHD, AMaZE, DCB, DHT, LMMSE, and VNG; report root
  causes and propose fixes without implementing them.

## Files expected to change

- This plan, updated after each phase with its short report.
- Phase 1: `crates/fornax-raw-pipeline/tests/demosaic_invariants.rs` only.
- Phase 2: `compare-demosaic` ground-truth/report/diff/readme/run code,
  setup scripts, `baseline/canonical.json`, and generated README regions.
- Phase 3: `compare-demosaic` run/report/status/readme code, baseline, and
  generated README regions.
- Phase 4: this plan only.

## Validation

- Phase 0: `scripts/agent-validation.sh --docs-only`; run the existing
  `compare-demosaic` README/schema tests and the existing fast demosaic tests.
- Phase 1: targeted `cargo test -p fornax-raw-pipeline`, formatting, clippy, and
  normal workspace validation for the changed crate.
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

## Phase 1 invariant tests

`crates/fornax-raw-pipeline/tests/demosaic_invariants.rs` contains seven
table-driven tests over all eleven algorithms. They use a fixed seeded LCG, a
64x64 mosaic, GBRG for the general checks, and all four Bayer patterns for
constant fields, shift behavior, and horizontal-flip symmetry.

Floating-point comparisons use relative tolerance `2e-4` plus absolute floor
`1e-7`. This allows accumulated `f32` roundoff while retaining large failures:
for comparison, the scale failures in the matrix are mostly caused by code
quantization or fixed epsilons and differ by much more than this tolerance.

The matrix is executable. A `Fail` entry means the test requires that failure to
continue so a later behavior change cannot silently erase the finding. If an
algorithm starts passing, the test fails and the matrix must be reviewed.

| Algorithm | Scale | Shift | Flip | Constant | Ramp | Tiling | Finite | Negative | HDR |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| AAHD | Fail | N/A | Fail | Fail | N/A | Pass | Pass | Fail | Fail |
| AHD | Fail | N/A | Pass | Pass | N/A | Pass | Pass | Fail | Fail |
| AMaZE | Fail | N/A | Fail | Pass | N/A | Pass | Pass | Fail | Fail |
| DCB | Fail | N/A | Fail | Pass | N/A | Pass | Pass | Fail | Fail |
| DHT | Fail | N/A | Fail | Pass | N/A | Pass | Pass | Fail | Fail |
| Linear | Pass | Pass | Pass | Pass | Pass | Pass | Pass | Pass | Pass |
| LMMSE | Fail | N/A | Pass | Fail | N/A | Pass | Pass | Fail | Fail |
| PPG | Pass | N/A | Pass | Pass | N/A | Pass | Pass | Fail | Pass |
| RCD | Fail | N/A | Fail | Pass | N/A | Pass | Pass | Fail | Pass |
| VNG | Fail | N/A | Pass | Pass | Pass | Pass | Pass | Fail | Fail |
| VNG4 | Fail | N/A | Pass | Pass | Pass | Pass | Pass | Fail | Fail |

Notable observations:

- All algorithms produce finite output for negative, all-zero, `1e4`-scale, and
  `1e6`-scale deterministic mosaics at 64x64.
- AAHD fails whole-image constant preservation at the border. LMMSE fails it in
  the interior as well as at tile boundaries.
- Horizontal flip symmetry passes for AHD, LMMSE, Linear, PPG, VNG, and VNG4 at
  the stated tolerance. AAHD, AMaZE, DCB, DHT, and RCD have direction or border
  differences.
- PPG and RCD preserve negative measured CFA samples but floor reconstructed
  samples at zero, so a constant negative field becomes a mix of `-0.25` and
  zero rather than a negative image.
- A preliminary 32x32 run exposed an AMaZE index-out-of-bounds panic before the
  invariant matrix could continue. The general property size was moved to 64x64
  so that all algorithms are measured; the size-boundary panic remains a
  separate Phase 4 finding.

### Phase 1 validation

- `cargo test -p fornax-raw-pipeline`: 89 unit tests, 7 invariant tests, 1
  public-API test, and 6 doctests passed.
- `cargo +nightly fmt -p fornax-raw-pipeline -- --check`: passed.
- `cargo clippy -p fornax-raw-pipeline --test demosaic_invariants -- -D warnings
  -A clippy::too_many_arguments`: passed.
- `scripts/agent-validation.sh`: passed when run outside the sandbox. The
  sandboxed attempt stopped at `tracker-duckdb` because Pixi could not acquire
  its global cache lock; the unrestricted rerun passed that stage.
- A stricter ad hoc clippy command without the legacy `too_many_arguments`
  exception failed on eight existing AMaZE/LMMSE function signatures. The
  repository-configured clippy stage passed.

## Phase 2 ground-truth harness

Canonical schema version 2 adds a `ground_truth` section. The section records
its own version, canonical pattern, parameters, synthetic source names, corpus
availability, and one row per algorithm. `Report::validate` checks the schema,
ground-truth parameters, ranks, metrics, and worst coordinates. Full `diff`
fails when ground truth changes; `--verdict-only` continues to ignore
numeric-only changes. Partial runs compare only their selected ground-truth
algorithms and the required Linear floor.

Ground-truth generation uses 128x128 GBRG data and an 8-pixel border:

- Six deterministic synthetic scenes: zone plate, slanted edge, colored edges,
  fine lines, smooth gradient, and HDR blocks from `2^-8` to `16`.
- Three fixed corpus files, decoded at run time, cropped to 128x128, binned by
  their native 2x2 Bayer quads to 64x64 RGB truth, nearest-expanded back to
  128x128, re-mosaiced, and demosaiced.
- Corpus paths are `Canon/EOS 5D Mark III/M24-1221_000202.dng`,
  `Sigma/fp/A002_653_20240229_000001_2k_8bit.DNG`, and
  `Nikon/D80/DSC_0143.dng`. The setup scripts materialize only these LFS
  objects. Runtime marks them evaluated, missing, or too small.

Metrics are scene-linear:

- Relative error divides by `max(|truth|, |output|, 1e-3)`.
- Log2 error floors both truth and output at `1e-3`.
- Zipper is relative luma error on interior edge pixels.
- False color is the magnitude of output-minus-truth `R-G`/`B-G` chroma,
  normalized by luma.
- Aggregate relative/log/edge metrics exclude the 8-pixel border. Worst pixels
  scan the whole frame and report edge, flat, or border class.
- An algorithm is suspicious if a zone-plate or edge scene has a relative mean
  more than 1% worse than Linear.

Final aggregate ranking:

| Rank | Algorithm | Relative mean | Relative p99 | Relative p99.9 | Log2 mean | Zipper | False color | Vs Linear | Suspicious scenes |
| ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| 1 | RCD | 0.065586 | 0.931822 | 0.975703 | 0.194254 | 0.126813 | 0.444795 | 0.886x | none |
| 2 | PPG | 0.067768 | 0.931838 | 1.000000 | 0.203377 | 0.129063 | 0.462044 | 0.916x | colored edges |
| 3 | Linear | 0.073991 | 0.931821 | 0.939347 | 0.213540 | 0.144554 | 0.486986 | 1.000x | floor |
| 4 | DHT | 0.093205 | 0.931859 | 0.937500 | 0.277186 | 0.131179 | 0.415780 | 1.260x | none |
| 5 | LMMSE | 0.099534 | 0.931853 | 0.937500 | 0.295187 | 0.135028 | 0.453509 | 1.345x | colored edges |
| 6 | AAHD | 0.100549 | 0.931878 | 1.000000 | 0.300270 | 0.136247 | 0.461066 | 1.359x | colored edges |
| 7 | AMaZE | 0.100583 | 0.931894 | 1.000000 | 0.301081 | 0.136620 | 0.464231 | 1.359x | colored edges |
| 8 | AHD | 0.100721 | 0.931911 | 1.000000 | 0.301827 | 0.135293 | 0.463964 | 1.361x | colored edges |
| 9 | DCB | 0.101240 | 0.931890 | 1.000000 | 0.303590 | 0.134674 | 0.457065 | 1.368x | colored edges |
| 10 | VNG | 0.103990 | 0.931853 | 0.984253 | 0.304693 | 0.147521 | 0.450235 | 1.405x | colored edges |
| 11 | VNG4 | 0.103990 | 0.931853 | 0.984253 | 0.304693 | 0.147521 | 0.450235 | 1.405x | colored edges |

Literature expects advanced directional/MMSE algorithms to beat bilinear on
edge detail. Only RCD and PPG do so in this aggregate, while DHT and the rest are
materially worse; that is a surprise and remains visible rather than being
tuned away. Every non-Linear worst pixel is at a border. Because aggregate
metrics exclude the border, those worst locations are separate border evidence,
not the cause of the ranking.

The README contains the generated table, metric definitions, corpus status,
suspicious-scene list, and literature comparison. The heavy all-algorithm and
corpus checks are marked `#[ignore]`; they do not run during default
`cargo test`. Synthetic scene generation, binning, summary math, ranking,
selection-aware diffing, schema rejection, and README generation remain in the
normal fast suite.

### Phase 2 validation

- `cargo test -p compare-demosaic`: 153 library tests and 3 binary tests
  passed; the two heavy ground-truth tests were skipped as intended.
- The heavy tests passed explicitly with `--ignored`: deterministic ranking and
  all three fixed corpus files.
- Full canonical run: 76 reference rows plus 11 ground-truth rows completed;
  canonical schema version 2 and ground-truth version 1 were written.
- `compare-demosaic diff` over the refreshed full baseline reports no
  differences.
- A partial `--algorithm linear --reference libraw --pattern GBRG` run also
  diffs cleanly against the full baseline.
- `compare-demosaic readme --check` reports the README is up to date.
- Nightly package rustfmt and `cargo insta test -p compare-demosaic` passed.
- `cargo clippy -p compare-demosaic --all-targets` completed with only the
  existing `too_many_arguments` warning in `artifacts::write`.
- `scripts/agent-validation.sh` passed, including workspace tests, rustdoc,
  workspace/compare clippy, benchmark compilation, compare setup, and compare
  tests. An earlier run failed because the historical corpus submodule commit
  was no longer available upstream; the setup path now reuses an existing
  checkout or initializes current `master` and fetches only the three required
  LFS files.
