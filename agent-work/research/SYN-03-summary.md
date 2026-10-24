# RAW Pipeline Research Synthesis

> **Task:** `SYN-03`
> **Owner:** primary agent
> **Review:** GitHub task pull request; its open/merged state is authoritative.
> **Pull request:** [#546](https://github.com/Glatzel/fornax/pull/546).
> **Last updated:** 2026-10-24
>
> **Prerequisites:** `RES-01`–`RES-09`, `SYN-01`, and `SYN-02`, all merged.
>
> **Acceptance criteria (authoritative in the task manifest):**
>
> - [x] Every reference report is represented and linked.
> - [x] Agreements and disagreements are explicit, with conflicting evidence.
> - [x] Recommendations are labeled and traceable to report sections.
> - [x] Remaining questions and assumptions are listed with impact.
> - [x] The summary synthesizes rather than concatenates.
> - [x] No unreferenced claim is introduced.
> - [x] Implementation recommendations weight LibRaw, dnglab, and darktable
>   evidence without treating it as normative format law.

## 1. Purpose and method

This report synthesizes the nine reference reports and the two reconciliation
notes into a cross-source recommendation for `fornax`. It organizes the
evidence by architectural concern, records where sources agree or conflict, and
separates settled recommendations from design gates.

The synthesis does not add upstream evidence. Every material recommendation
links to the owning report or note.

Authority is applied consistently:

- normative DNG semantics govern DNG payloads;
- source-verified LibRaw and dnglab behavior describes actual backend
  observations;
- darktable contributes preprocessing and pipeline-order evidence;
- vendor/catalog data retains provenance; and
- a `fornax` adapter must reject a translation it cannot perform without
  changing meaning.

This follows the authority model in
[SYN-01 §Scope and synthesis rules](notes/SYN-01.md#scope-and-synthesis-rules).

## 2. Source coverage

| Source | Contribution to this synthesis |
| --- | --- |
| [RES-01 — Existing `fornax` inventory](RES-01-fornax-inventory.md) | Current crate/API limitations and reusable assets; no existing public contract should constrain the replacement. |
| [RES-02 — LibRaw](RES-02-libraw.md) | Unpacked raw buffer, stride, CFA, black/white levels, crop origins, mutating correction hazards, and licensing. |
| [RES-03 — dnglab and DNG](RES-03-dnglab.md) | DNG tag semantics, sample encoding, default crop, ActiveArea, CFA origin, black tiles/deltas, opcodes, and rawler boundary. |
| [RES-04 — darktable](RES-04-darktable.md) | `rawprepare` numerical order, CFA-aware correction, clipping behavior, demosaic boundary, and later-processing exclusions. |
| [RES-05 — Demosaicing algorithms](RES-05-demosaicing.md) | Input assumptions, border policies, output semantics, algorithm tradeoffs, and initial algorithm shortlist. |
| [RES-06 — Rust RAW decoders](RES-06-rust-raw-decoders.md) | rawloader/rawler representation, coverage, native integer/float payloads, mutation, maintenance, and licenses. |
| [RES-07 — Rust TIFF/DNG building blocks](RES-07-rust-dng-tiff.md) | image-tiff capability/limits, DNG-facing rawler pieces, and parser dependency tradeoffs. |
| [RES-08 — Rust demosaicing](RES-08-rust-demosaicing.md) | `demosaic` and MeraRAW API contracts, f32 interfaces, borders, validation, tests, and licenses. |
| [RES-09 — Selectable decoder backends](RES-09-rust-ecosystem.md) | Adapter strategy, backend roles, source-domain preservation, provenance, and dependency/license decisions. |
| [SYN-01 — Metadata/sample semantics](notes/SYN-01.md) | Reconciled decision and conflict tables for samples, levels, metadata, coordinates, CFA, and state. |
| [SYN-02 — Preprocessing](notes/SYN-02.md) | Numeric operation sequence, state transitions, errors, precision, clipping, and synthetic tests. |

Deferred scope is consistent across the reports: conventional 2×2 RGB Bayer
only; DNG, Sony ARW, Nikon NEF, and Canon `CR*` are initial targets; X-Trans,
Foveon/X3, Quad Bayer, four-color, monochrome, LinearRaw, and non-mosaic payloads
must be structured unsupported outcomes
([RES-01 §2.3](RES-01-fornax-inventory.md#23-bayer-model);
[RES-03 §1](RES-03-dnglab.md#1-scope-and-questions);
[RES-06 §4](RES-06-rust-raw-decoders.md#4-implications-for-fornax)).

## 3. Architecture and decoder boundary

`[recommendation]` Build the pipeline around a backend-independent RAW model and
selectable decoder adapters. Do not implement or maintain a first-party RAW
format decoder in the initial scope.

- LibRaw is the primary existing C/C++ decoder backend; use `unpack`, snapshot
  metadata, and do not call its post-processing path
  ([RES-02 §2.2–§2.8](RES-02-libraw.md#22-decode-lifecycle-and-buffer-meaning);
  [RES-09 §2.4](RES-09-rust-ecosystem.md#24-libraw-adapter)).
- dnglab/rawler is the primary pure Rust backend and has the broadest target
  dispatch, including CR3. Do not invoke `apply_scaling`
  ([RES-06 §2.4](RES-06-rust-raw-decoders.md#24-rawler);
  [RES-09 §2.5](RES-09-rust-ecosystem.md#25-dnglabrawler-adapter)).
- The existing `dnc` wrapper is a conversion bridge to a DNG decoder, not a
  decoder itself
  ([RES-01 §4](RES-01-fornax-inventory.md#4-implications-for-fornax);
  [RES-09 §2.6](RES-09-rust-ecosystem.md#26-dnc-conversion-backend)).
- darktable is preprocessing evidence and a test oracle; application output is
  outside the decode boundary. A rawspeed adapter requires an isolated
  decoder-only API
  ([RES-04 §2.7–§2.8](RES-04-darktable.md#27-relationship-to-libraw-and-dng);
  [RES-09 §2.7](RES-09-rust-ecosystem.md#27-darktablerawspeed-boundary)).

Assumption: backend adapters can preserve stored sensor samples and required
metadata without entering their downstream image pipelines. If a target backend
cannot do so, it fails the adapter gate.

## 4. Metadata, sample precision, and provenance

`[recommendation]` Decode returns source samples, not a normalized image.
Initial native storage supports u8, u16, u32, and f32 containers. Declared bit
depth and sample format remain metadata rather than being inferred from
container width
([SYN-01 §Decision table](notes/SYN-01.md#decision-table);
[RES-02 §2.2](RES-02-libraw.md#22-decode-lifecycle-and-buffer-meaning);
[RES-03 §2.2](RES-03-dnglab.md#22-dng-tag-mapping)).

`[recommendation]` Provide an explicit f32 conversion on `DecodedRaw` for user
custom processing before standard preprocessing; conversion returns the same
`Decoded` state. The resulting sample basis must be recorded, and stale
linearization/black/white metadata cannot be applied silently afterward
([SYN-02 §Numerical model](notes/SYN-02.md#numerical-model)).

`[recommendation]` Keep algorithm-critical metadata typed and provenance-bearing.
Use an extensible namespaced map only for supplemental/vendor/unknown values.
File declarations, decoder observations, camera-catalog fallbacks, defaults,
and derived/measured values must remain distinguishable
([SYN-01 §Decision table](notes/SYN-01.md#decision-table);
[RES-02 §2.7](RES-02-libraw.md#27-metadata-retention-and-the-safe-wrapper);
[RES-03 §2.5](RES-03-dnglab.md#25-dng-versus-vendor-raw-metadata)).

`[recommendation]` Preserve raw dimensions, compact buffer dimensions, source
row stride, ActiveArea, DefaultCrop, masked areas, and orientation separately.
CFA and black-repeat phase are anchored to ActiveArea, never to a cropped
buffer origin
([SYN-01 §Decision table](notes/SYN-01.md#decision-table);
[RES-02 §2.3–§2.5](RES-02-libraw.md#23-dimensions-stride-and-crop-coordinates);
[RES-03 §2.3](RES-03-dnglab.md#23-coordinate-spaces-and-cfa-phase)).

## 5. Preprocessing and demosaic boundary

`[recommendation]` Use explicit staged preprocessing:

```text
decoded source
  -> validated stored image
  -> optional DecodedRaw.convert_to_f32/custom operation with metadata-basis decision
  -> optional linearization
  -> ActiveArea/CFA-aware black subtraction
  -> linear-reference normalization
  -> explicit final clipping
  -> f32 demosaic-ready mosaic
```

This follows [SYN-02 §Proposed processing sequence](notes/SYN-02.md#proposed-processing-sequence)
and the darktable/LibRaw evidence reconciled in
[SYN-02 §Operation table](notes/SYN-02.md#operation-table).

`[recommendation]` Use f32 preprocessing buffers and demosaic input/output.
Every initial ≤16-bit integer is exact in f32; use f64 scalar arithmetic for
black/white subtraction and normalization to avoid avoidable rounding. f64
image buffers are not required and double memory/bandwidth. Unclipped corrected
and normalized f32 states remain available; final demosaic input is clamped to
`[0,1]`
([SYN-02 §Numerical model](notes/SYN-02.md#numerical-model)).

`[recommendation]` Make stage transitions consume the previous typed state so
linearization or black correction cannot be applied twice. Backend-normalized
input is rejected unless a future task explicitly accepts it
([SYN-02 §Processing-state constraints](notes/SYN-02.md#processing-state-constraints);
[RES-02 §2.5](RES-02-libraw.md#25-black-level-representation);
[RES-04 §2.7](RES-04-darktable.md#27-relationship-to-libraw-and-dng)).

`[recommendation]` End the required pipeline after demosaicing. Do not apply WB,
camera color matrices, highlight reconstruction, gain maps, tone mapping,
denoising, sharpening, color-space conversion, or display rendering
([RES-04 §2.6](RES-04-darktable.md#26-demosaic-boundary-and-later-processing);
[SYN-02 §Demosaic boundary](notes/SYN-02.md#demosaic-boundary)).

## 6. Demosaicing

`[recommendation]` Expose a stable Bayer strategy boundary and implement
bilinear first, then Malvar–He–Cutler. Bilinear is an existing reusable
baseline; MHC gives a deterministic quality/cost improvement with explicit
border and overshoot policy. Edge-aware algorithms follow only after their
options and quality tests are independently justified
([RES-05 §2.5](RES-05-demosaicing.md#25-algorithm-selection)).

`[recommendation]` Use f32 planar output as the initial algorithm-facing
contract. The selected Rust `demosaic` entry accepts f32 mosaic input and
caller-owned planar f32 output and validates algorithm/CFA combinations; do not
expose its CFA/error types publicly
([RES-08 §2.1–§2.2](RES-08-rust-demosaicing.md#21-demosaic-crate-api-and-bayer-contract);
[SYN-02 §Demosaic boundary](notes/SYN-02.md#demosaic-boundary)).

Metadata reference and channel/border semantics must cross the demosaic
boundary. Higher-order algorithms may undershoot or overshoot; their output
clipping policy must be explicit rather than inherited from a backend
([RES-05 §2.4](RES-05-demosaicing.md#24-output-semantics)).

## 7. Dependency and license recommendations

| Candidate | Recommendation | Constraint |
| --- | --- | --- |
| LibRaw | Optional/default existing decoder adapter | LGPL-2.1 or CDDL-1.0 decision required; use `unpack` only |
| dnglab/rawler | Preferred pure Rust decoder adapter | LGPL-2.1; pre-1.0; pin version, contain panics, no `apply_scaling` |
| image-tiff | DNG/TIFF building-block fallback | MIT; still needs a DNG semantic layer and target-fixture proof |
| `demosaic` crate | Adapter-level demosaicing implementation | MIT OR Apache-2.0; validate buffers before panic-prone helpers |
| MeraRAW | Design reference/test oracle | Apache-2.0; pipeline-wide types and stubs make it unsuitable as initial core |
| darktable/rawspeed | Design reference; isolated decoder experiment only | GPL-3.0 application; decoder-only and license boundary unresolved |
| `dnc` | Optional conversion bridge | Windows/macOS external Adobe runtime and terms require review |

These recommendations consolidate
[RES-09 §2.2](RES-09-rust-ecosystem.md#22-candidate-decision-matrix) and
[RES-09 §2.9](RES-09-rust-ecosystem.md#29-license-and-maintenance-decisions).
No upstream RAW type enters the universal core.

## 8. Agreements across sources

| Agreement | Evidence |
| --- | --- |
| Decode should preserve source-domain samples; black correction/normalization belong later | [RES-02 §2.2–§2.5](RES-02-libraw.md#22-decode-lifecycle-and-buffer-meaning); [RES-03 §2.1–§2.4](RES-03-dnglab.md#21-decode-representation-and-sample-domain); [RES-04 §2.1–§2.3](RES-04-darktable.md#21-current-raw-pipeline-order) |
| CFA phase and black selection need source/ActiveArea coordinates | [SYN-01 §Decision table](notes/SYN-01.md#decision-table); [SYN-02 §Black-level representation](notes/SYN-02.md#black-level-representation) |
| Black metadata cannot safely collapse to one scalar for all sources | [RES-02 §2.5](RES-02-libraw.md#25-black-level-representation); [RES-03 §2.2](RES-03-dnglab.md#22-dng-tag-mapping); [RES-04 §2.2–§2.4](RES-04-darktable.md#22-rawprepare-inputs-and-output) |
| Metadata and decoder provenance must survive each stage | [SYN-01 §Decision table](notes/SYN-01.md#decision-table); [RES-06 §2.2–§2.5](RES-06-rust-raw-decoders.md#22-maintenance-and-public-surface) |
| WB/color matrices must not be applied implicitly | [RES-04 §2.1 and §2.6](RES-04-darktable.md#21-current-raw-pipeline-order); [RES-02 §4](RES-02-libraw.md#4-implications-for-fornax) |
| Initial scope is conventional Bayer; deferred layouts require rejection | [RES-03 §4](RES-03-dnglab.md#4-implications-for-fornax); [RES-06 §4](RES-06-rust-raw-decoders.md#4-implications-for-fornax) |
| f32 is the practical demosaic interface for the initial normalized data | [RES-08 §2.1](RES-08-rust-demosaicing.md#21-demosaic-crate-api-and-bayer-contract); [SYN-02 §Demosaic boundary](notes/SYN-02.md#demosaic-boundary) |

## 9. Disagreements and resolutions

| Conflict | Competing evidence | Resolution | Status |
| --- | --- | --- | --- |
| Negative/overflow clipping | DNG recommends clipping >1 and permits negatives; rawler clips negatives/retains overflow; darktable clips neither in `rawprepare` | Do not inherit a backend policy. Preserve unclipped f32 stages and clamp to `[0,1]` explicitly at demosaic-ready output. [SYN-02](notes/SYN-02.md#numerical-model) | Resolved |
| Black-level shape | DNG supports repeat tiles/deltas; LibRaw canonicalizes several forms; rawler/darktable summarize four slots | Keep a general model; support 1×1/2×2 canonicalization initially; reject larger tiles/deltas until origin behavior is implemented. [SYN-01](notes/SYN-01.md#conflict-log) | Partially resolved |
| Black repeat origin after crop | DNG anchors at ActiveArea; LibRaw spatial correction can index crop-local coordinates | ActiveArea origin is authoritative; reject backend paths that cannot preserve it. [RES-02 §2.5](RES-02-libraw.md#25-black-level-representation); [RES-03 §2.3](RES-03-dnglab.md#23-coordinate-spaces-and-cfa-phase) | Resolved policy; backend tests pending |
| White level meaning | DNG declares per-plane saturation; LibRaw `maximum` may be adjusted and also exposes linear/measured maxima | Preserve declared, adjusted, and measured values separately with provenance. [SYN-01](notes/SYN-01.md#decision-table) | Resolved for representation |
| CFA plane mapping | DNG has `CFAPlaneColor`/`CFALayout`; rawler assumes common RGB in its DNG path | Initial scope accepts conventional RGB only and rejects alternatives. [RES-03 §2.2 and §5](RES-03-dnglab.md#22-dng-tag-mapping) | Resolved |
| Sample precision | Native integer containers may be u8/u16/u32; floating paths use f32; f32 preprocessing is not universally higher precision | Support u8/u16/u32/f32 storage, preserve bit depth, use f32 preprocessing when source precision is exactly representable, and reject/widen wider cases. [SYN-02](notes/SYN-02.md#numerical-model) | Resolved for initial scope |
| DNG parser approach | image-tiff provides generic TIFF but not DNG semantics; rawler provides DNG support with a broad mutable model | Use rawler as a decoder adapter initially; retain image-tiff as fallback building block after fixture proof. [RES-07 §2.3](RES-07-rust-dng-tiff.md#23-dependency-versus-minimal-parser); [RES-09 §2.4](RES-09-rust-ecosystem.md#24-libraw-adapter); [RES-09 §2.5](RES-09-rust-ecosystem.md#25-dnglabrawler-adapter) | Recommendation; fixture gate |
| Demosaic quality | `demosaic` and MeraRAW have different interfaces/status; no independent comparative quality ranking | Choose algorithms by contract/testability first: bilinear, then MHC. Do not claim quality superiority from crate names. [RES-05 §2.2](RES-05-demosaicing.md#22-candidate-comparison); [RES-08 §2.1–§2.5](RES-08-rust-demosaicing.md#21-demosaic-crate-api-and-bayer-contract) | Deferred quality validation |

## 10. Recommendations and invalidation conditions

| `[recommendation]` | Confidence | Assumptions | Invalidated if |
| --- | --- | --- | --- |
| Backend-neutral adapter architecture; no first-party decoder | High | Existing backend sample/metadata APIs can be isolated | Target files require format behavior unavailable through acceptable backends |
| Universal source image with native u8/u16/u32/f32 and separate bit depth | Medium-high | u32 support requires decoder/source evidence; f32 conversion is lossless only within its significand | Fixtures expose required >32-bit integer or f64 semantics |
| Typed metadata plus provenance-aware extension map | High | Backend adapters can identify source/default/catalog origin | A backend cannot expose enough origin to avoid ambiguity |
| ActiveArea-relative CFA and black coordinates | High | Backends can preserve ActiveArea origin or compact faithfully | Required target cameras expose irreconcilable coordinate semantics |
| f32 preprocessing/demosaic, f64 scalar intermediates | High | Target inputs normalize to `[0,1]` with ≤16-bit source precision | High-precision source/output requirements exceed f32 |
| Explicit stage typestate and reject already-corrected input | High | Operations can consume/return state values | FFI boundary prevents reliable state transport |
| Bilinear then MHC initial demosaic set | Medium | Existing implementations/tests can be adapted behind one f32 interface | Acceptance fixtures show unacceptable quality or API mismatch |
| LibRaw/dnglab optional adapters; dnc bridge; darktable reference | Medium-high | License/platform decisions are acceptable | License review rejects a backend or target coverage fails |

## 11. Open questions and impact

| Question | Impact | Blocking gate |
| --- | --- | --- |
| Which real Bayer DNG/ARW/NEF/CR2/CR3 fixtures pass each backend? | Source dispatch is not support proof; sample/CFA/level semantics remain unverified | Decoder implementation and QA |
| Can general spatial black tiles and BlackLevelDeltaH/V be represented and applied correctly? | Required files must be rejected until exact ActiveArea-origin behavior exists | Preprocessing design/implementation |
| Which DNG opcodes/gain maps are required by the target corpus? | Required unsupported transformations must reject; optional ones must not run implicitly | Preprocessing/decoder design |
| Which image-tiff compression/predictor/opcode combinations cover Bayer DNG? | Determines whether image-tiff is a viable fallback parser | DNG adapter design |
| How are fractional DefaultCrop/DefaultScale semantics represented? | Integer truncation loses crop geometry | Universal model/coordinate design |
| How should vendor catalog/file metadata conflicts resolve per field? | Wrong authority can change calibration/levels without changing pixels | Metadata design |
| What transformed metadata must custom f32 processing supply? | Stale black/white metadata could be applied after arbitrary user changes | Preprocessing API design |
| How do candidate demosaicers compare on target fixtures and quality metrics? | Algorithm selection cannot rely on names or upstream claims | Demosaic design/QA |
| What are final LibRaw/rawler/darktable dependency and distribution terms? | LGPL/GPL/external-runtime constraints may change packaging | License/release review |
| Do backend decoders silently transform samples before exposure? | Universal stored-domain assumptions would be false | Adapter capability and fixture tests |

## 12. References

Research reports:

- [RES-01 — Existing `fornax` inventory](RES-01-fornax-inventory.md)
- [RES-02 — LibRaw](RES-02-libraw.md)
- [RES-03 — dnglab and DNG metadata](RES-03-dnglab.md)
- [RES-04 — darktable preprocessing](RES-04-darktable.md)
- [RES-05 — Demosaicing algorithms](RES-05-demosaicing.md)
- [RES-06 — Rust RAW decoders](RES-06-rust-raw-decoders.md)
- [RES-07 — Rust TIFF/DNG building blocks](RES-07-rust-dng-tiff.md)
- [RES-08 — Rust demosaicing](RES-08-rust-demosaicing.md)
- [RES-09 — Selectable decoder backends](RES-09-rust-ecosystem.md)

Synthesis notes:

- [SYN-01 — Metadata and sample semantics](notes/SYN-01.md)
- [SYN-02 — Preprocessing and black levels](notes/SYN-02.md)

## Acceptance checklist

- [x] Every reference report is represented and linked.
- [x] Agreements and disagreements are explicit, with conflicting evidence.
- [x] Recommendations are labeled and traceable to report sections.
- [x] Remaining questions and assumptions are listed with impact.
- [x] The summary synthesizes rather than concatenates.
- [x] No unreferenced claim is introduced.
- [x] Implementation recommendations weight LibRaw, dnglab, and darktable
  evidence without treating it as normative format law.
