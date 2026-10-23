# Rust Ecosystem Comparison and Reuse Recommendation

> **Task:** `RES-09`
> **Owner:** primary agent
> **Review:** GitHub task pull request; its open/merged state is authoritative.
> **Pull request:** [#537](https://github.com/Glatzel/fornax/pull/537).
> **Last updated:** 2026-10-24
> **Upstream versions inspected through split reports:** rawloader
> `8f2ace2d27c7fa6e7671a7fd0ef11d1b1bd77ddf`; rawler as part of dnglab
> `62b77de1a206d467e910d55199191068b45a9cc1`; image-tiff
> `3302936b08e2260d4c66f40f51170efbb1fded4c`; `demosaic`
> `5050131e998b7f6414eef02eeb60f74b99d77451`; MeraRAW Demosaic
> `b94ebd4324ce56218e42989a5ed852b32a52003f`.
>
> **Acceptance criteria (authoritative version in the task TOML manifest):**
>
> - [x] Every candidate is compared on scope, Bayer and target-format fit, API
>   boundary, maintenance, and license.
> - [x] Every recommendation links to a split report.
> - [x] Dependency, optional-backend, and design-reference status is explicit.
> - [x] No upstream RAW type is recommended for `fornax-core` without an
>   adapter and license decision.
> - [x] Remaining unknowns and their impact are listed.
> - [x] The report contains the six required research sections and cites the
>   split reports.
>
> **Verification snapshot:** RES-09 comparison, 2026-10-24. The three split
> reports were reviewed at merged commit
> `20efbe98c4f14ecd000ef7f14c424b45ded02426` after PR #534. This report
> compares their evidence and does not re-establish their source-level claims.

## 1. Scope and questions

This report compares the Rust ecosystem findings owned by
[`RES-06-rust-raw-decoders.md`](RES-06-rust-raw-decoders.md),
[`RES-07-rust-dng-tiff.md`](RES-07-rust-dng-tiff.md), and
[`RES-08-rust-demosaicing.md`](RES-08-rust-demosaicing.md). It does not repeat
their upstream source research, settle DNG tag semantics, or choose the final
public API.

The comparison is constrained to the initial pipeline: a single-plane,
conventional 2x2 RGB Bayer mosaic with an explicit CFA phase, decoded from DNG,
Sony ARW, Nikon NEF, or Canon `CR*` files. X-Trans, Foveon/X3, Quad Bayer,
four-color, monochrome, non-mosaic payloads, and other RAW formats remain
unsupported until separately approved.

The questions were:

- Which candidates are suitable as dependencies, optional backends, design
  references, or not reusable?
- Does each candidate preserve the initial Bayer and target-format scope
  without moving deferred sensor layouts into `fornax-core`?
- Where must an adapter translate upstream types, preprocessing state, errors,
  and coordinate metadata?
- What maintenance and license obligations constrain adoption?
- Which unknowns must be closed by fixtures or design work before a candidate
  becomes required?

## 2. Findings

### 2.1 Comparison criteria

The candidates are judged on five criteria:

1. **Scope fit:** how much of decoding, DNG parsing, or demosaicing the
   component actually owns.
2. **Bayer and target-format fit:** verified conventional Bayer behavior and
   source-dispatch coverage for DNG, ARW, NEF, and Canon `CR*`, including CR3.
3. **API boundary:** whether upstream sample and RAW types can be converted at
   an adapter without leaking mutable preprocessing state or panics into the
   universal pipeline.
4. **Maintenance:** inspected version, recent revision, release stability, and
   evidence of tests. None of the split reports found a published long-term
   support policy.
5. **License:** the declared license and the review obligation it creates.

Source-presence evidence is not fixture evidence. In particular, `RES-06` and
`RES-07` did not execute target RAW fixtures, so target-format support below is
dispatch coverage unless explicitly called out otherwise.

### 2.2 Candidate decision matrix

| Candidate | Scope fit | Bayer and target-format fit | API boundary | Maintenance | License | RES-09 decision |
| --- | --- | --- | --- | --- | --- | --- |
| rawloader 0.37.2 | Rust RAW decoder with a broad format catalog | Source dispatch for DNG, ARW, NEF, CR2, and CRW; no CR3/BMFF path; conventional Bayer is represented through a general CFA type | Returns its own `RawImage`/`RawImageData`; no explicit stride, typed sample domain, active-area type, or arbitrary black-level model; string errors | Inspected revision dated 2026-08-16; public decode path converts decoder panics to errors | LGPL-2.1 | **Design reference, not an initial dependency/backend.** Incomplete metadata and absent CR3 make it a fallback catalog/CFA-shift reference, not the preferred adapter. |
| rawler 0.8.0 | Rust RAW decoder integrated with dnglab | Source dispatch covers DNG, ARW, NEF, CR2, CRW, and CR3; richest target metadata model; conventional and deferred CFAs coexist in its broader type | Owns a broad mutable `RawImage`; `apply_scaling` mutates linearization and normalization state, and some lower-level paths panic; typed public errors help the boundary | Inspected revision dated 2026-10-23; pre-1.0; broad tests/implementation surface but no target fixtures or LTS policy | LGPL-2.1 | **Preferred optional decoder backend**, behind a version-pinned adapter and Cargo feature. Do not use its RAW type in `fornax-core` or call its mutating scaling path inside the universal pipeline. |
| image-tiff 0.11.3 | Generic TIFF decoder, not a DNG-semantic layer | Provides the strips/tiles, samples, tags, strides, limits, and structured errors needed to build a Bayer DNG adapter; no verified Bayer DNG fixture and no native DNG metadata semantics | Generic `Decoder`, tag entries, sample layouts, and typed errors adapt cleanly to an owned DNG adapter; CFA/crop/black/white interpretation remains `fornax` work | Inspected revision dated 2026-09-06; recent activity and strong generic TIFF tests; no DNG-specific tests or LTS policy | MIT | **Direct dependency inside a dedicated DNG adapter**, preferred over a minimal parser after target-fixture coverage is established. Compression features remain explicit and reviewable. |
| `demosaic` crate | Bayer demosaicing library with several interchangeable algorithms | All four 2x2 RGB Bayer phases; validates exact planar buffer sizes and algorithm/CFA pairs; no RAW decoding or DNG/vendor-format responsibility | Accepts a single-channel row-major `f32` mosaic and caller-owned planar `f32` output; typed errors cover the primary entry, while some interleaved conversion helpers still panic on malformed buffers | Inspected revision dated 2026-02-18; dedicated phase, border, dispatch, and error tests; no independently reproduced quality ranking in `RES-08` | MIT OR Apache-2.0 | **Direct dependency inside the demosaicing adapter** for the initial candidate algorithm set, with strict prevalidation and no direct exposure of its CFA/error types. |
| MeraRAW Demosaic | Demosaicing crate plus broader image pipeline and algorithm registry | Bayer adapters exist and unsupported CFA checks are explicit; several advertised strategies are stubs; broader pipeline includes stages outside the fornax boundary | Trait operates on MeraRAW `RawImage` and returns `RgbImage`, then adapts through a legacy `CfaImage` path with copies; allocation failure is not reliably surfaced despite an `Alloc` variant | Inspected revision dated 2026-07-22; strong synthetic quality/tiling tests and explicit statuses; no target RAW integration evidence in `RES-08` | Apache-2.0 | **Design reference, not an initial dependency.** Reuse its capability registry, validation ideas, border/tiling cases, and synthetic quality harness without importing its pipeline-wide types. |

The candidate rows derive from [RES-06 §2.1–§2.5](RES-06-rust-raw-decoders.md#21-target-format-evidence),
[RES-07 §2.1–§2.3](RES-07-rust-dng-tiff.md#21-image-rsimage-tiff), and
[RES-08 §2.1–§2.5](RES-08-rust-demosaicing.md#21-demosaic-crate-api-and-bayer-contract).

### 2.3 Raw decoder boundary

`[recommendation]` rawler is the only inspected Rust decoder with source-level
coverage for every required file family, including Canon CR3. It should be an
optional backend in an adapter crate, selected by an explicit Cargo feature and
pinned by version. The adapter must:

- decode without invoking `RawImage::apply_scaling`;
- convert samples, rational black levels, active/default crops, CFA plane
  mapping, orientation, matrices, tags, and provenance into independent
  `fornax` types;
- record original sample domain and preprocessing state rather than assuming
  values are already normalized;
- translate errors and isolate known panic preconditions; and
- reject deferred layouts with a structured unsupported-layout error.

This recommendation is traceable to [RES-06 §2.2–§2.5](RES-06-rust-raw-decoders.md#22-maintenance-and-public-surface)
and [RES-07 §2.2](RES-07-rust-dng-tiff.md#22-rawlers-dng-facing-pieces).
rawler's richer model is an advantage only if the adapter snapshots it before
any potentially mutating operation.

`[recommendation]` rawloader should remain a design reference for its camera
catalog, cleaned camera names, and CFA crop shifting. Its lack of CR3, fixed
four-level metadata, general CFA surface, and untyped sample domain make it a
poorer initial backend than rawler. It may be reconsidered as a narrower
fallback only if fixture testing exposes a format-specific advantage.
Evidence remains in [RES-06 §2.3–§2.5](RES-06-rust-raw-decoders.md#23-rawloader).

### 2.4 DNG and TIFF reuse

`[recommendation]` Use image-tiff as the parser dependency for a first-party
Bayer DNG adapter. Its MIT license and generic TIFF plumbing avoid making the
project own IFD, strip/tile, tag, stride, and error machinery. It does not
justify skipping a DNG layer: photometric interpretation, CFA validation,
active/default crop mapping, black/white semantics, opcode handling, and
provenance stay in `fornax`.

Adoption is conditional on a target-fixture gate covering the required Bayer
DNG subset. That gate must verify:

- `PhotometricInterpretation` and sample format/bit depth;
- strip and tile organization, row/plane stride, and any predictor;
- each enabled compression family;
- relevant opcodes and the initial decision to accept or reject them;
- CFA, active area, default crop, orientation, black-level, and white-level
  extraction; and
- configured limits on allocation, IFD values, and decoded intermediates.

Keep optional compression features off until the corpus requires them. Compare
the fixture result with the build-versus-buy analysis in
[RES-07 §2.1 and §2.3](RES-07-rust-dng-tiff.md#21-image-rsimage-tiff). A
minimal parser remains a fallback, not the default, because it transfers TIFF
compatibility and malformed-input hardening into this project.

rawler also supplies a complete DNG path. It remains an optional backend rather
than the preferred DNG building block because its RAW model, preprocessing
mutation, and LGPL-2.1 boundary are broader than the adapter needs
([RES-07 §2.2](RES-07-rust-dng-tiff.md#22-rawlers-dng-facing-pieces)).
The design should not decode through both layers in the same path; the DNG
adapter chooses image-tiff, while a later rawler backend may decode DNG only
through its own backend route.

### 2.5 Demosaicing boundary

`[recommendation]` Use the `demosaic` crate inside a `fornax` demosaicing
adapter for the initial algorithm set after `RES-05` and design choose that set.
The adapter should prevalidate image geometry, exact buffer lengths, CFA phase,
supported algorithm, and output layout before calling upstream code. It should
allocate or accept the output according to the `fornax` policy and should not
expose upstream `CfaPattern`, `Algorithm`, or `DemosaicError` as public types.

The primary upstream entry provides useful validation and caller-owned planar
output. Helpers with panic-prone layout conversion are not part of the trusted
adapter surface. Border behavior belongs to each strategy's documented and
tested contract. These constraints come from
[RES-08 §2.1–§2.4](RES-08-rust-demosaicing.md#21-demosaic-crate-api-and-bayer-contract).

`[recommendation]` MeraRAW is a design reference for capability registration,
implementation status, options separation, border/tiling tests, and synthetic
quality metrics. It is not the initial dependency: its trait consumes its own
`RawImage`/`RgbImage`, its adapter adds copies, some allocation failures are not
reliably returned, many registry entries are stubs, and its broader pipeline
orders white balance before demosaicing, which is outside `fornax` scope
([RES-08 §2.3–§2.5](RES-08-rust-demosaicing.md#23-meraraw-demosaic-api-and-validation)).

### 2.6 License and maintenance decisions

`[recommendation]` Keep all upstream use at adapter boundaries:

- `fornax-core` must not expose rawloader, rawler, image-tiff, `demosaic`, or
  MeraRAW types.
- The rawler optional backend carries an LGPL-2.1 review before release
  packaging. Selecting it does not authorize copying its code or linking
  decisions; the adapter feature must retain a license and notice decision in
  design review.
- The rawloader design reference remains LGPL-2.1. Any move from reference to
  code reuse requires the same review.
- image-tiff (MIT) and `demosaic` (MIT OR Apache-2.0) require retention of their
  license and notice terms. MeraRAW is Apache-2.0; using its code rather than
  its interface ideas would also require checking NOTICE obligations.
- Pin exact versions and lockfile revisions. None of the split reports found a
  published LTS policy, and rawler is pre-1.0.

The license facts and maintenance evidence are summarized in
[RES-06 §2.2](RES-06-rust-raw-decoders.md#22-maintenance-and-public-surface),
[RES-07 §2.1–§2.2](RES-07-rust-dng-tiff.md#21-image-rsimage-tiff), and
[RES-08 §2.1–§2.5](RES-08-rust-demosaicing.md#21-demosaic-crate-api-and-bayer-contract).

## 3. Evidence

This report intentionally cites split-report sections rather than repeating
upstream file and line evidence.

| Conclusion | Supporting split-report evidence |
| --- | --- |
| rawler has the broadest target-format dispatch, including CR3 | [RES-06 §2.1](RES-06-rust-raw-decoders.md#21-target-format-evidence) |
| rawler's metadata is rich but its mutation and panic surface needs an adapter | [RES-06 §2.4](RES-06-rust-raw-decoders.md#24-rawler); [RES-07 §2.2](RES-07-rust-dng-tiff.md#22-rawlers-dng-facing-pieces) |
| rawloader lacks CR3 and sufficient typed metadata for the initial backend | [RES-06 §2.3](RES-06-rust-raw-decoders.md#23-rawloader) |
| image-tiff has suitable generic TIFF plumbing but no DNG semantics or Bayer DNG fixture proof | [RES-07 §2.1](RES-07-rust-dng-tiff.md#21-image-rsimage-tiff) |
| image-tiff versus rawler versus a minimal parser has the stated tradeoffs | [RES-07 §2.3](RES-07-rust-dng-tiff.md#23-dependency-versus-minimal-parser) |
| the `demosaic` crate has a usable Bayer contract and typed primary errors, with panic-prone conversion helpers | [RES-08 §2.1–§2.2](RES-08-rust-demosaicing.md#21-demosaic-crate-api-and-bayer-contract) |
| MeraRAW offers useful interface/test ideas but brings a broader pipeline and unresolved allocation behavior | [RES-08 §2.3–§2.5](RES-08-rust-demosaicing.md#23-meraraw-demosaic-api-and-validation) |
| all upstream RAW types must remain behind adapters | [RES-06 §4](RES-06-rust-raw-decoders.md#4-implications-for-fornax); [RES-07 §4](RES-07-rust-dng-tiff.md#4-implications-for-fornax); [RES-08 §4](RES-08-rust-demosaicing.md#4-implications-for-fornax) |

The dependency gate used here is the merged aggregate PR
[#534](https://github.com/Glatzel/fornax/pull/534), whose merge commit contains
RES-06 through RES-08 and the RES-09 manifest. The RES-09 contract remains
`agent-work/tasks/raw-pipeline/RES-09.toml`.

## 4. Implications for fornax

`[recommendation]` The proposed component map is:

| `fornax` boundary | Selected ecosystem role | Required isolation |
| --- | --- | --- |
| Universal RAW model in `fornax-core` | No upstream RAW type | Owned backend-independent samples, metadata, coordinates, and processing state |
| Rust vendor-format decoder backend | rawler optional backend | Feature-gated adapter; samples copied into the universal model; no mutating scaling |
| First-party Bayer DNG adapter | image-tiff direct dependency | DNG semantics and support gates remain owned by `fornax`; gated compression features |
| Demosaicing interface and implementations | `demosaic` adapter dependency | Owned Bayer/input/output/error types; validated call into upstream algorithms |
| Registry/test design | MeraRAW design reference | Reuse ideas and test patterns, not its pipeline types or application order |

This keeps decoding, metadata interpretation, preprocessing, and demosaicing
separate. It also gives the design a permissive DNG path, a broader LGPL
optional backend when required, and a permissive Bayer algorithm dependency
without making any upstream RAW representation canonical.

`[recommendation]` Support claims remain fixture-driven. Source dispatch is enough
to select candidates, not to state that DNG, ARW, NEF, CR2/CR3/CRW decode
correctly or preserve metadata. The implementation plan must include target
fixtures and explicit rejection tests for deferred layouts. Until then, the
initial recommendation is an architecture shortlist, not a release-support
claim.

## 5. Open questions and limitations

- No inspected rawler, rawloader, or image-tiff checkout included target RAW
  fixtures. Impact: decoder and Bayer DNG acceptance cannot be inferred from
  source dispatch; fixture work remains mandatory.
- The required Bayer DNG compression, predictor, opcode, strip, and tile matrix
  has not been measured with image-tiff. Impact: image-tiff is the preferred
  parser, but unsupported combinations may force an explicit rejection,
  feature choice, or fallback parser.
- rawler's complete error, panic, and opcode surface has not been audited.
  Impact: the optional adapter must contain known calls and add malformed-input
  regressions before enabling the backend.
- Neither decoder report established camera-level support inside each format.
  Impact: support should be reported by tested camera/file model rather than by
  file extension alone.
- No candidate has a documented long-term support policy, and rawler is a
  pre-1.0 dependency. Impact: pin releases and keep adapter boundaries small
  enough to replace or patch a backend.
- Comparative image quality was not independently reproduced for either Rust
  demosaicing implementation. Impact: `demosaic` is a code candidate, while
  the algorithm set and quality acceptance remain `RES-05`/design decisions.
- MeraRAW currently reports multiple registered strategies as stubs and does
  not reliably surface allocation failure despite an `Alloc` error variant.
  Impact: registry names cannot be treated as implemented capabilities, and
  allocation behavior must not be inherited.
- The exact LGPL-2.1 packaging and notice decision for an optional rawler
  backend remains a release-review question. Impact: an adapter dependency
  recommendation does not remove the need for a license decision before
  distribution.
- CFA phase, active/default crop, and black-level translation depend on
  synthesis and design decisions not owned by RES-09. Impact: these mappings
  are requirements on the adapters, not finalized APIs in this report.

## 6. References

Split reports:

- [RES-06 — Rust RAW decoder crates](RES-06-rust-raw-decoders.md)
- [RES-07 — Rust TIFF and DNG building blocks](RES-07-rust-dng-tiff.md)
- [RES-08 — Rust demosaicing crates](RES-08-rust-demosaicing.md)
- [RES-09 task manifest](../tasks/raw-pipeline/RES-09.toml)

Upstream projects at the revisions compared above:

- rawloader: <https://github.com/pedrocr/rawloader/tree/8f2ace2d27c7fa6e7671a7fd0ef11d1b1bd77ddf>
- rawler / dnglab: <https://github.com/dnglab/dnglab/tree/62b77de1a206d467e910d55199191068b45a9cc1>
- image-tiff: <https://github.com/image-rs/image-tiff/tree/3302936b08e2260d4c66f40f51170efbb1fded4c>
- `demosaic`: <https://github.com/naorunaoru/demosaic/tree/5050131e998b7f6414eef02eeb60f74b99d77451>
- MeraRAW Demosaic: <https://github.com/kaimai-apex/meraraw-demosaic/tree/b94ebd4324ce56218e42989a5ed852b32a52003f>

Source paths, symbols, line ranges, and license-file evidence remain in the
corresponding split report and are not duplicated here.
