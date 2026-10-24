# SYN-02 — Preprocessing Order and Black-Level Behavior

> **Task:** `SYN-02`
> **Owner:** primary agent
> **Review:** GitHub task pull request; its open/merged state is authoritative.
> **Last updated:** 2026-10-24
>
> **Prerequisites:** `RES-02`, `RES-04`, `SYN-01`
>
> **Output consumed by:** `SYN-03`, design tasks `DES-001`, `DES-004`, `DES-005`.
>
> **Acceptance criteria (authoritative in the task manifest):**
>
> - [x] An operation table defines inputs, outputs, formulas, error behavior,
>   ordering constraints, and synthetic tests.
> - [x] Black-level behavior is numerically explicit.
> - [x] Missing, invalid, and inconsistent metadata behavior is specified.
> - [x] Processing-state semantics prevent accidental double correction.
> - [x] Every decision links to a supporting report or SYN-01 item.
> - [x] Implementation decisions explicitly weight LibRaw and darktable as
>   high-priority references while preserving documented disagreements.

## Scope and authority

This note defines the sensor-domain preprocessing contract between decoded
stored samples and demosaicing. It does not define the final Rust buffer type,
the decoder backend trait, or demosaicing algorithms.

The authority order from [SYN-01](SYN-01.md#scope-and-synthesis-rules)
continues here:

- normative DNG semantics govern DNG interpretation;
- LibRaw and darktable provide high-priority implementation evidence;
- their different correction and clipping behavior is explicit rather than
  inherited; and
- unresolved or unrepresentable required metadata produces a structured error.

The initial scope is conventional 2×2 RGB Bayer DNG, Sony ARW, Nikon NEF, and
Canon `CR*`. Other layouts and unsupported preprocessing operations are
rejected.

## Proposed processing sequence

```text
decoded stored samples
  -> validate metadata and processing state
  -> optional linearization-table application
  -> select black from ActiveArea-relative metadata
  -> exact/widened black subtraction
  -> normalization to DNG linear-reference meaning
  -> explicit clamping to [0.0, 1.0]
  -> f32 demosaic-ready mosaic
```

The data types are proposed as distinct states; implementations must consume
one state and produce the next. The design gate may rename them, but mutable
in-place correction without a state transition is not acceptable.

## Numerical model

For an active sample at stored coordinate `(r, c)`:

```text
l       = L(stored_sample)
b       = black_common + black_slot[slot(r,c)] + black_tile[tile(r,c), plane]
q       = l - b
range   = white[slot(r,c)] - b
n       = q / range
d       = round_to_f32(clamp(n, 0.0, 1.0))
```

- `L` is identity when no linearization table is present; otherwise it is the
  declared table lookup.
- `slot(r,c)` derives from `(r - active.top) mod 2`,
  `(c - active.left) mod 2`, never from cropped-buffer-local coordinates.
- `tile(r,c)` derives from `(r - active.top) mod repeat_rows`,
  `(c - active.left) mod repeat_cols`.
- Absent black components contribute exact zero only when the source declares
  their default or the adapter proves absence.
- `b` and `range` are evaluated in `f64` after exact integer/relational
  validation. Integer source samples up to 32 bits are exactly representable
  in `f64`.
- `q` and `n` are unclipped working values. Negative and above-one values are
  retained until the final clipping operation.
- `d` is single-channel `f32` in `[0.0, 1.0]`. The final `f64` to `f32`
  conversion uses IEEE round-to-nearest, ties-to-even.
- NaN and infinity are errors, not clampable values.

The `[0.0, 1.0]` final clamp is chosen for the initial demosaic-ready input.
The unclipped normalized state remains available to diagnostics and tests.
DNG permits retaining negative normalized values; this implementation chooses
to clamp only at the explicit final boundary rather than claiming DNG requires
the clamp.

## Black-level representation

The universal formula uses additive components:

```text
black_common
  + black_slot[4]                 (zero when absent)
  + black_tile[tile_row, tile_col, plane]  (zero when absent)
```

Adapter conversion must ensure these components are additive and preserve
their source. A DNG declared tile may be represented wholly by `black_tile`
with the other components zero; it must not be divided into arbitrary
components without a recorded derivation.

For conventional 1×1 and 2×2 Bayer patterns, LibRaw's canonical scalar,
per-CFA, and folded tile values may be represented by common and per-slot
components when the decomposition is exact. A larger spatial tile or
row/column delta remains unsupported in the initial pipeline and is rejected
if it affects the accepted sample. This preserves the distinction in
[SYN-01](SYN-01.md#decision-table) instead of collapsing every black value to
four numbers.

## Operation table

| Order | Operation | Input domain | Output domain | Required metadata | Formula / rule | Error behavior | Synthetic tests |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | Validate decode state | Backend-produced stored candidate | `ValidatedStoredRaw` | Sample encoding, dimensions, stride, CFA layout, ActiveArea, black/white candidates, provenance | Reject already-corrected/normalized samples; validate every required field, rectangle, stride, encoding, and layout invariant | `UnsupportedPreprocessingState`, `MissingRequiredMetadata`, `InvalidGeometry`, `UnsupportedCfa`, or `MetadataConflict`; no silent defaults for unknown semantics | Valid stored state; missing CFA; already normalized; wrong stride; malformed ActiveArea; unsupported layout |
| 2 | Apply linearization table | `ValidatedStoredRaw`; declared table | `LinearizedRaw`; table treated as identity when absent | Table entries, source encoding/range, provenance | `l = table[stored_code]` or `l = stored_code`; do not use an out-of-range index | `UnsupportedLinearization` or `InvalidMetadata`; no extrapolation | Identity path; nontrivial table; first/last entry; out-of-range code; malformed table |
| 3 | Select black level | `LinearizedRaw` plus ActiveArea-relative CFA coordinate | Per-sample black tuple with provenance | Common, per-slot, and repeat components; tile dimensions/origin; white level for later validation | Select slot from active coordinates; select tile offset from active coordinates; sum only declared additive components | `UnsupportedBlackPattern`, `InvalidBlackLevel`, or `MissingRequiredMetadata` if a non-default component is required | Scalar black; four-slot phases; 1×1 tile at odd ActiveArea origin; 2×2 tile; general tile rejection; absent component |
| 4 | Subtract black | Linearized sample and selected black | Unclipped signed `BlackCorrectedRaw` | Valid selected black and selected white level | `q = l - b` in widened arithmetic; no clamp at this stage | `InvalidMetadata` when black is negative or not below white for required normalization; invalid numeric values abort | Positive result; zero result; negative underflow retained; maximum sample; black equal white |
| 5 | Normalize | `BlackCorrectedRaw` and selected white level | Unclipped `NormalizedRaw` | `white`, `black`, valid nonzero range, CFA plane | `n = (l - b) / (white - b)` in `f64`; preserve sign and overflow | `InvalidMetadata` when `white <= b`, denominator is zero, or values are non-finite | Black maps to zero; white maps to one; underflow below zero; value above one; floating rounding fixture |
| 6 | Clip to demosaic range | `NormalizedRaw` | `DemosaicReadyMosaic` | Explicit policy version, CFA/ActiveArea geometry | `d = clamp(n, 0.0, 1.0)`; convert to `f32` using round-to-nearest, ties-to-even | `PreprocessingStateError` if input is not normalized; non-finite values are rejected before clipping | Negative clamps to zero; exact one; above one clamps to one; f64-to-f32 tie rounding; NaN/Infinity rejection |
| 7 | Validate demosaic boundary | `DemosaicReadyMosaic` | Immutable demosaic input | Single active Bayer grid, stride/contiguity policy, CFA layout, processing-state tag | Verify `[0.0, 1.0]`, finite f32 samples, dimensions, and ActiveArea phase; no correction may follow | `PreprocessingStateError`, `InvalidGeometry`, or `NonFiniteSample` | All four Bayer phases; wrong state; wrong dimensions; non-finite sample; unchanged geometry after cropping |

Operation evidence:

| Operation | Supporting evidence |
| --- | --- |
| Validate decode state | [SYN-01](SYN-01.md#decision-table); [RES-02 §2.2](../RES-02-libraw.md#22-decode-lifecycle-and-buffer-meaning) |
| Apply linearization | [RES-04 §4](../RES-04-darktable.md#4-implications-for-fornax); [SYN-01](SYN-01.md#decision-table) |
| Select black level | [RES-02 §2.5](../RES-02-libraw.md#25-black-level-representation); [RES-04 §2.2–§2.4](../RES-04-darktable.md#22-rawprepare-inputs-and-output) |
| Subtract black | [RES-02 §2.5](../RES-02-libraw.md#25-black-level-representation); [RES-04 §2.3](../RES-04-darktable.md#23-numerical-black-level-correction-and-normalization) |
| Normalize | [SYN-01](SYN-01.md#decision-table); [RES-04 §2.3](../RES-04-darktable.md#23-numerical-black-level-correction-and-normalization) |
| Clip | [SYN-01 conflict log](SYN-01.md#conflict-log); [RES-04 §4](../RES-04-darktable.md#4-implications-for-fornax) |
| Validate demosaic boundary | [RES-04 §2.6–§2.7](../RES-04-darktable.md#26-demosaic-boundary-and-later-processing); [SYN-01](SYN-01.md#decision-table) |

## Required, optional, and unsupported operations

| Operation | Initial status | Reason |
| --- | --- | --- |
| Linearization table | Required when the accepted source declares one; otherwise identity | DNG/rawler evidence places it before black correction |
| Scalar and four-slot black subtraction | Required | Verified conventional Bayer behavior in LibRaw, rawler, and darktable |
| Conventional 1×1/2×2 repeat tile | Required only after adapter proves exact decomposition and ActiveArea alignment | Existing backends fold or summarize these values differently |
| General spatial black tile | Unsupported initial | LibRaw origin behavior and rawler/darktable representations are incomplete |
| BlackLevelDeltaH/V | Unsupported unless exactly represented and applied | LibRaw averages them; rawler does not parse them |
| Normalization to `[0,1]` | Required | DNG linear-reference meaning; darktable and rawler broadly agree |
| Final clipping | Required policy at demosaic boundary | Backends disagree, so `fornax` owns the explicit final policy |
| Masked-pixel black estimation | Optional and unsupported initially | Inspected rawspeed fallback is described as broken; no verified `fornax` policy |
| DNG gain maps / OpcodeList1/2 | Unsupported unless separately approved | Optional format behavior can change samples before/after normalization |
| Hot-pixel/defect correction, raw denoise, CA correction | Unsupported initial | Separate operations requiring evidence and tests |
| Highlight reconstruction, WB, camera matrices, tone mapping, color-space conversion, output sharpening/denoising | Excluded from pipeline | Outside decode/preprocess/demosaic boundary |
| DNG OpcodeList3 / demosaic-time behavior | Not part of preprocessing | Stage begins after the demosaic-ready boundary |

## Error behavior and metadata failures

| Condition | Required result |
| --- | --- |
| Backend already reports corrected or normalized samples | Structured unsupported-processing-state error unless explicitly accepted by a future task |
| CFA layout is not conventional 2×2 RGB Bayer | Structured unsupported-layout error before correction |
| ActiveArea or CFA phase is missing or inconsistent | Structured metadata error; never derive from buffer `(0,0)` |
| Black default is absent | Use one declared default only when the source format supplies it; otherwise error |
| White level is absent | Use the DNG default only when derivable from encoding; otherwise error |
| Black component is negative or not representable in the selected numeric domain | Structured invalid-metadata error |
| `white <= black` | Invalid metadata; no normalization |
| General spatial tile or nonzero delta vectors are required but unsupported | Structured unsupported-black-level error |
| A component may exist but adapter cannot read it | Capability error; do not substitute zero |
| Source values are non-finite | Invalid-sample error before arithmetic |
| Same-authority required metadata disagrees | Structured metadata conflict per SYN-01 |

## Processing-state constraints

The stage types consume ownership or use a sealed transition token:

```text
DecodedRaw
  -> ValidatedStoredRaw
  -> LinearizedRaw
  -> BlackCorrectedRaw
  -> NormalizedRaw
  -> DemosaicReadyMosaic
```

Requirements:

- no downstream function accepts `DecodedRaw` where a normalized mosaic is
  required;
- an attempted transition from the wrong state is a compile-time error where
  practical and a structured runtime error at FFI/backend boundaries;
- corrected/normalized state is never inferred from zero black metadata or a
  float buffer type;
- a linearization table or black correction can occur once per transition; and
- cloned values retain the state, so copying cannot reset the correction stage.

This follows the double-correction warnings in
[SYN-01](SYN-01.md#decision-table),
[RES-02 §2.5](../RES-02-libraw.md#25-black-level-representation), and
[RES-04 §2.7](../RES-04-darktable.md#27-relationship-to-libraw-and-dng).

## Demosaic boundary

Demosaicing receives:

- one immutable single-channel Bayer mosaic over ActiveArea dimensions;
- a validated conventional 2×2 RGB `BayerLayout` with original ActiveArea
  origin/phase;
- row stride and sample-range policy;
- `f32` finite values in `[0.0, 1.0]`;
- `DemosaicReadyMosaic` processing state; and
- preserved metadata/provenance needed after demosaicing without reapplying
  preprocessing.

Demosaicing must not:

- subtract black again;
- apply a linearization table;
- apply WB or a camera color matrix;
- reconstruct highlights or apply gain maps;
- crop by resetting CFA phase; or
- mutate the preprocessed mosaic in place.

The boundary is after final clipping and before any demosaic algorithm. Any
algorithm-specific range or border requirement is handled by the demosaic
adapter without changing preprocessing semantics.

## Synthetic test plan

The implementation task should include a focused matrix:

1. **Identity transform:** stored code equals white; black zero; normalized
   output is exactly `1.0`.
2. **Scalar black:** three samples below, at, and above black produce the
   expected unclipped normalized values and final clamped values.
3. **Four-slot black:** an odd ActiveArea origin selects each of `R`, `G1`,
   `G2`, and `B` correctly across the four Bayer phases.
4. **Repeat tile:** 1×1 and 2×2 tiles produce the same value when rebased to
   stored coordinates; a non-zero origin catches local-coordinate mistakes.
5. **Crop invariance:** selecting an ActiveArea subset changes dimensions but
   not the black/CFA phase at the same stored coordinates.
6. **Linearization:** a two-entry and a full-width lookup table produce exact
   expected mapping; out-of-range lookup errors.
7. **Underflow:** a sample below black remains negative in normalized state and
   clamps to zero only at the final stage.
8. **Overflow:** a sample above white remains above one in normalized state and
   clamps to one only at the final stage.
9. **Invalid metadata:** missing/ambiguous CFA, `white <= black`, unsupported
   tile/delta, and conflicting source values return the specified structured
   errors.
10. **State safety:** applying subtraction or normalization twice cannot be
    expressed through the state API; backend-normalized input is rejected.
11. **Round-trip provenance:** every operation carries source metadata and
    transformation identity into the demosaic-ready state.

## Open questions

- The final implementation's exact widened arithmetic type is for `DES-004`; it
  must preserve the numerical results specified here for the initial sample
  encodings.
- A verified black-level fallback from masked pixels may be added later only
  after separate evidence and an explicit user-approved task.
- General spatial black tiles and BlackLevelDeltaH/V remain unsupported until
  exact ActiveArea-origin behavior and backend capability reporting are
  implemented.
- DNG opcode and embedded gain-map acceptance remains a separate format/semantic
  decision; no opcode is enabled implicitly.
- Default white level for uncommon encodings and non-integer sample depths
  requires fixture validation.
- Whether diagnostics retain the unclipped normalized buffer after
  demosaic-ready conversion is a design/memory tradeoff for `DES-004`.

## Evidence

- [SYN-01 decision table](SYN-01.md#decision-table): sample states, units,
  coordinates, CFA, black/white metadata, provenance, and processing state.
- [SYN-01 conflict log](SYN-01.md#conflict-log): correction order, black shape,
  origin, clipping, deltas, and double correction.
- [RES-02 §2.2–§2.6](../RES-02-libraw.md#22-decode-lifecycle-and-buffer-meaning):
  LibRaw unpack/raw-to-image lifecycle, stride, black subtract, and white
  level.
- [RES-04 §2.1–§2.4](../RES-04-darktable.md#21-current-raw-pipeline-order):
  darktable `rawprepare` order, formula, CFA phase, and crop behavior.
- [RES-04 §2.6–§2.7](../RES-04-darktable.md#26-demosaic-boundary-and-later-processing):
  demosaic boundary and later excluded processing.

## Acceptance checklist

- [x] Operation table complete with inputs, outputs, formulas, ordering, and
  test strategy.
- [x] Black-level representation and CFA-aware selection defined.
- [x] Subtraction, clipping, underflow, saturation, normalization, and rounding
  specified numerically.
- [x] Boundary before demosaicing explicit; later operations named as excluded.
- [x] Missing, invalid, and inconsistent metadata behavior specified.
- [x] Processing-state semantics prevent double correction.
- [x] Every decision links to a supporting report or SYN-01 item.
