# Raw Decoder Backend and Reuse Recommendation

> **Task:** `RES-09`
> **Owner:** primary agent
> **Review:** GitHub task pull request; its open/merged state is authoritative.
> **Pull request:** [#537](https://github.com/Glatzel/fornax/pull/537).
> **Last updated:** 2026-10-24
> **Upstream versions inspected through split reports:** LibRaw `0.22.2`
> (`b93f6e45c194f5df9b02a43b1af9a54b4f41f33f`); darktable
> `6556cd7dbb0e69f95d165dda8b69e7270c327f6d`; rawloader
> `8f2ace2d27c7fa6e7671a7fd0ef11d1b1bd77ddf`; rawler as part of dnglab
> `62b77de1a206d467e910d55199191068b45a9cc1`; image-tiff
> `3302936b08e2260d4c66f40f51170efbb1fded4c`; `demosaic`
> `5050131e998b7f6414eef02eeb60f74b99d77451`; MeraRAW Demosaic
> `b94ebd4324ce56218e42989a5ed852b32a52003f`.
>
> **Acceptance criteria (authoritative version in the task TOML manifest):**
>
> - [x] Every decoder, conversion bridge, and reusable component is compared on
>   scope, Bayer and target-format fit, API boundary, maintenance, and license.
> - [x] Every recommendation links to a split report.
> - [x] Dependency, selectable-backend, conversion-bridge,
>   experimental-adapter, and design-reference status are explicit.
> - [x] A backend-neutral adapter contract requires explicit selection,
>   capabilities, structured rejection, and provenance.
> - [x] No upstream RAW type is recommended for `fornax-core` without an
>   adapter and license decision.
> - [x] The recommendation does not require `fornax` to maintain a first-party
>   RAW decoder.
> - [x] Remaining unknowns and their impact are listed.
> - [x] The report contains the six required research sections and cites the
>   split reports.
>
> **Verification snapshot:** RES-09 comparison, 2026-10-24. The decoder,
> preprocessing, TIFF/DNG, and demosaicing findings in RES-01 through RES-08
> were reviewed at merged commit
> `20efbe98c4f14ecd000ef7f14c424b45ded02426` after PR #534. This report
> compares their evidence and does not re-establish their source-level claims.

## 1. Scope and questions

This report turns the split-report findings into an initial ecosystem decision:
`fornax` should not implement or maintain a RAW format decoder. It should define
one backend-neutral decode contract and adapt existing decoder implementations
to it. Backend-specific findings remain owned by
[`RES-01`](RES-01-fornax-inventory.md), [`RES-02`](RES-02-libraw.md),
[`RES-03`](RES-03-dnglab.md), and [`RES-04`](RES-04-darktable.md); the Rust
offerings remain owned by [`RES-06`](RES-06-rust-raw-decoders.md),
[`RES-07`](RES-07-rust-dng-tiff.md), and
[`RES-08`](RES-08-rust-demosaicing.md). This report does not repeat their
source evidence, settle DNG tag semantics, or choose the final public API.

Treating "reuse an existing decoder" as the architectural constraint changes
the earlier candidate ordering. The comparison now asks which existing decoder
can be wrapped without leaking its image type, processed pixels, or mutable
preprocessing state into `fornax`.

The comparison is constrained to the initial pipeline: a single-plane,
conventional 2x2 RGB Bayer mosaic with an explicit CFA phase, decoded from DNG,
Sony ARW, Nikon NEF, or Canon `CR*` files. X-Trans, Foveon/X3, Quad Bayer,
four-color, monochrome, non-mosaic payloads, and other RAW formats remain
unsupported until separately approved.

The questions were:

- Which decoders should be selectable backends rather than targets for a
  first-party parser or vendored decoder?
- Which candidates are suitable as dependencies, optional backends, design
  references, conversion bridges, or not reusable?
- Does each candidate preserve the initial Bayer and target-format scope
  without moving deferred sensor layouts into `fornax-core`?
- Where must an adapter translate upstream types, preprocessing state, errors,
  and coordinate metadata?
- How should callers select a backend without silently changing sample
  semantics or falling through to a lossy processor?
- What maintenance and license obligations constrain adoption?
- Which unknowns must be closed by fixtures or design work before a candidate
  becomes required?

## 2. Findings

### 2.1 Comparison criteria and architecture constraint

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

`[recommendation]` `fornax` owns the universal sample model, metadata semantics,
backend capability description, structured errors, and deterministic backend
selection. It does not own TIFF/RAW format decoding. A backend adapter must stop
at decoded sensor-domain data; it must not expose output from a complete RAW
developer or invoke the backend's black correction, normalization, white
balance, demosaicing, or display pipeline.

### 2.2 Candidate decision matrix

| Candidate | Scope fit | Bayer and target-format fit | API boundary | Maintenance | License | RES-09 decision |
| --- | --- | --- | --- | --- | --- | --- |
| LibRaw 0.22.2 | Established decoder plus later RAW-to-image processing | Broad mature camera catalog; DNG, ARW, NEF, and Canon RAW are supported by its decoder model. Exact target-camera fixtures remain unverified locally | `unpack` exposes a borrowed native sample buffer and rich metadata; the raw buffer must be copied with `raw_pitch`, active-area origin, CFA, and black/white metadata before any mutating call | Actively maintained; the current wrapper omits key metadata and has stride/pointer defects, but its ownership plumbing is reusable behind a new adapter | LGPL-2.1 or CDDL-1.0 | **Preferred first decoder backend adapter.** Do not expose LibRaw types in `fornax-core`; call `unpack`, never `dcraw_process`; replace or harden the current wrapper boundary. |
| dnglab/rawler 0.8.0 | Pure Rust RAW decoder integrated with dnglab | Source dispatch covers DNG, ARW, NEF, CR2, CRW, and CR3; richest target metadata model; conventional and deferred CFAs coexist in its broader type | Owns a broad mutable `RawImage`; `apply_scaling` mutates linearization and normalization state, and some lower-level paths panic; typed public errors help the boundary | Inspected revision dated 2026-10-23; active but pre-1.0; broad tests/implementation surface but no target fixtures or LTS policy | LGPL-2.1 | **Preferred Rust decoder backend adapter.** Keep it optional and version-pinned; copy decoded state before any mutation and never call `apply_scaling` in the universal pipeline. |
| fornax `dnc` wrapper | Adobe DNG Converter process bridge, not a decoder | Converts vendor RAW to a mosaic DNG that a DNG backend can decode; linear-DNG mode is outside scope; wrapper is absent on Linux | Current `Dnc::convert` returns a path and shells out to Adobe's executable; it does not return samples or metadata. Must become a conversion step composed with LibRaw or dnglab/rawler | Existing wrapper is small, but platform-gated and currently unchecked for process exit status; Adobe executable availability and distribution terms are external | `dnc` crate has no declared license in its manifest; Adobe converter terms and redistribution need review | **Conditional conversion backend**, not a direct decoder. Use only when a DNG-producing adapter is explicitly selected; retain provenance and clean temporary outputs. |
| darktable/rawspeed | Full RAW developer with a separate decoding layer inside its image I/O path | Uses rawspeed for RAW decoding and supplies black/white/crop metadata; darktable then applies `rawprepare` and later processing that must not run in `fornax` | No stable decoder-only public boundary was established. The application's output is processed imagery, not a universal sensor buffer. An adapter is viable only against an isolated rawspeed decoder API | Mature and active, but the inspected report establishes pipeline behavior rather than an embeddable decoder ABI | darktable is GPL-3.0; rawspeed terms and the chosen linking boundary require separate review | **Conditional experimental decoder-only adapter or preprocessing/test oracle.** Do not adapt the darktable application pipeline; use it as evidence unless rawspeed can be isolated cleanly. |
| rawloader 0.37.2 | Rust RAW decoder with a broad format catalog | Source dispatch for DNG, ARW, NEF, CR2, and CRW; no CR3/BMFF path; conventional Bayer is represented through a general CFA type | Returns its own `RawImage`/`RawImageData`; no explicit stride, typed sample domain, active-area type, or arbitrary black-level model; string errors | Inspected revision dated 2026-08-16; public decode path converts decoder panics to errors | LGPL-2.1 | **Secondary optional adapter or design reference.** Incomplete metadata and absent CR3 prevent it from being the preferred backend. |
| image-tiff 0.11.3 | Generic TIFF parser, not a whole RAW decoder | Provides strips/tiles, samples, tags, strides, limits, and structured errors; DNG semantics and CFA/crop/black interpretation would still be first-party work | Clean generic Rust API, but using it directly recreates the DNG adapter `fornax` wants to avoid maintaining | Recent activity and strong generic TIFF tests; no DNG-specific tests or LTS policy | MIT | **Building block only, not an initial DNG decoder path.** Prefer dnglab/rawler for Bayer DNG. |
| `demosaic` crate | Bayer demosaicing library with several interchangeable algorithms | All four 2x2 RGB Bayer phases; validates exact planar buffer sizes and algorithm/CFA pairs; no RAW decoding responsibility | Accepts a single-channel row-major `f32` mosaic and caller-owned planar `f32` output; typed errors cover the primary entry, while some interleaved conversion helpers still panic on malformed buffers | Inspected revision dated 2026-02-18; dedicated phase, border, dispatch, and error tests; no independently reproduced quality ranking in `RES-08` | MIT OR Apache-2.0 | **Direct dependency inside the demosaicing adapter** for the initial candidate algorithm set, with strict prevalidation and no direct exposure of its CFA/error types. |
| MeraRAW Demosaic | Demosaicing crate plus broader image pipeline and algorithm registry | Bayer adapters exist and unsupported CFA checks are explicit; several advertised strategies are stubs; broader pipeline includes stages outside the fornax boundary | Trait operates on MeraRAW `RawImage` and returns `RgbImage`, then adapts through a legacy `CfaImage` path with copies; allocation failure is not reliably surfaced despite an `Alloc` variant | Inspected revision dated 2026-07-22; strong synthetic quality/tiling tests and explicit statuses; no target RAW integration evidence in `RES-08` | Apache-2.0 | **Design reference, not an initial dependency.** Reuse its capability registry, validation ideas, border/tiling cases, and synthetic quality harness without importing its pipeline-wide types. |

The decoder rows derive from [RES-01 §2.5](RES-01-fornax-inventory.md#25-libraw-wrapper),
[RES-02 §2.1–§2.8](RES-02-libraw.md#21-source-and-licensing),
[RES-03 §2.1–§2.6](RES-03-dnglab.md#21-decode-representation-and-sample-domain),
[RES-04 §2.1–§2.8](RES-04-darktable.md#21-current-raw-pipeline-order), and
[RES-06 §2.1–§2.5](RES-06-rust-raw-decoders.md#21-target-format-evidence). The
DNG and demosaicing rows additionally derive from
[RES-07 §2.1–§2.3](RES-07-rust-dng-tiff.md#21-image-rsimage-tiff) and
[RES-08 §2.1–§2.5](RES-08-rust-demosaicing.md#21-demosaic-crate-api-and-bayer-contract).

### 2.3 Common selectable-backend contract

`[recommendation]` Define one backend-neutral decoder boundary, conceptually a
`RawDecoderBackend` trait. The design tasks must finalize its exact types, but
the contract needs three operations:

```text
backend identity and capabilities
probe(input) -> supported / unsupported / unsupported-layout
decode(input, decode-config) -> universal decoded-raw state or structured error
```

The capabilities must be queryable before decode and include:

- supported file formats and any camera/model limits;
- conventional Bayer phases and sample encodings the adapter can normalize;
- whether the output is stored sensor codes or a backend-specific processed
  sample domain;
- metadata coverage for active area, default crop, CFA, black/white levels,
  orientation, and provenance;
- platform, external runtime, and Cargo-feature requirements; and
- adapter limitations that can lead to a structured rejection.

Backend selection must be explicit and deterministic. A caller may choose a
stable backend identifier, or request capabilities and use a documented
priority policy. The selected backend, conversion path, source revision, sample
domain, and any transformations must be recorded in decoder provenance. A
fallback is permitted only when the next backend advertises the requested
capability; it must not silently substitute processed data or apply a second
preprocessing stage.

Every adapter must:

- return an owned or lifetime-safe buffer converted into `fornax` types;
- preserve stride, raw dimensions, active-area origin, CFA phase, crop
  descriptors, and black/white metadata;
- snapshot immutable metadata before a backend mutates its image state;
- translate errors and isolate known panic preconditions; and
- reject deferred layouts with a structured unsupported-layout error.

This contract follows the universal-data and replay requirements in
[RES-02 §4](RES-02-libraw.md#4-implications-for-fornax),
[RES-03 §4](RES-03-dnglab.md#4-implications-for-fornax), and
[RES-06 §4](RES-06-rust-raw-decoders.md#4-implications-for-fornax). The exact
trait remains a design decision, not a commitment made by this report.

### 2.4 LibRaw adapter

`[recommendation]` Make LibRaw the first full decoder backend adapter.
LibRaw has the broadest mature camera coverage among the existing alternatives,
already has FFI and ownership plumbing in the repository, and exposes native
sensor samples through `rawdata.raw_image` before `raw2image` or
`dcraw_process`. It is the pragmatic baseline when the existing C/C++ decoder
should be reused rather than replaced.

The adapter should reuse the existing `libraw-sys` declarations and improve the
safe wrapper rather than expose it directly. Required work includes:

- use `unpack` as the decode endpoint and never call `dcraw_process`;
- copy the raw buffer using `raw_pitch`, rejecting layouts it cannot represent
  safely;
- expose black, per-CFA black, repeat-tile shape, white/saturation, active area,
  crop, CFA, and orientation metadata before any mutating correction;
- preserve the LibRaw owner for the complete lifetime of borrowed data;
- model decoder-specific sample transformations instead of assuming every
  `raw_image` has identical stored-code semantics; and
- reject non-conventional layouts even if LibRaw can process them.

The existing wrapper is evidence for reuse and for the adapter boundary, not a
production-ready decoder contract. Its `color4_image` pointer condition and
stride handling must be corrected, and its missing black/white/crop accessors
must be added behind the adapter. See
[RES-01 §2.5](RES-01-fornax-inventory.md#25-libraw-wrapper) and
[RES-02 §2.2–§2.8](RES-02-libraw.md#22-decode-lifecycle-and-buffer-meaning).

### 2.5 dnglab/rawler adapter

`[recommendation]` Make dnglab/rawler the preferred pure Rust decoder adapter
and the first choice for Bayer DNG when a Rust-only build is required. It is the
only inspected Rust decoder with source dispatch for DNG, ARW, NEF, CR2/CRW,
and CR3. It also preserves more target metadata at its decode boundary than
rawloader.

The adapter must not call `RawImage::apply_scaling`. It should copy the decoded
samples and metadata into `fornax` types first, then apply `fornax`
preprocessing. This prevents rawler's mutating linearization/black/normalize
operation from being mistaken for decode and preserves the original metadata
needed for provenance. Known preconditions around panic-prone lower-level APIs
must be contained at the adapter edge. The current rawler DNG path is also
limited to a four-value Bayer scaling path for some operations and does not
fully preserve general black patterns or row/column deltas; the adapter must
reject what the universal model cannot yet represent.

See [RES-03 §2.1–§2.5](RES-03-dnglab.md#21-decode-representation-and-sample-domain)
and [RES-06 §2.4](RES-06-rust-raw-decoders.md#24-rawler).

`[recommendation]` Keep rawloader as a secondary optional adapter only if target
fixtures expose a format-specific advantage. Its lack of CR3 and incomplete
typed metadata make it a weaker default than dnglab/rawler. This remains
traceable to [RES-06 §2.3–§2.5](RES-06-rust-raw-decoders.md#23-rawloader).

`[recommendation]` Do not build a first-party Bayer DNG decoder on image-tiff as
the initial path. That would recreate the DNG semantics and maintenance burden
the adapter strategy is intended to avoid. image-tiff remains a useful
building-block option if dnglab/rawler fails a required target fixture, as
described in [RES-07 §2.1–§2.3](RES-07-rust-dng-tiff.md#21-image-rsimage-tiff).

### 2.6 dnc conversion backend

`[recommendation]` Treat the existing `dnc` wrapper as a DNG conversion
backend, not as a `RawDecoderBackend`. `Dnc::convert` shells out to Adobe DNG
Converter and returns only an output path
(`crates/dnc/src/wrapper.rs:40-82`); it neither returns sensor samples nor
claims a sample domain. The wrapper is compiled only on non-Linux targets
(`crates/dnc/src/lib.rs:5-8`).

A usable dnc route is:

```text
vendor RAW -> Adobe DNG Converter -> mosaic DNG -> LibRaw or dnglab adapter
```

The conversion adapter must request a non-linear mosaic DNG, preserve source
and converter provenance, use a controlled temporary output location, verify
the process exit status, and delete temporary files after the selected DNG
backend has finished decoding. Linear-DNG mode is outside the initial pipeline
because it changes the sample representation. If dnc is selected with no
following DNG backend, selection must fail explicitly.

This classification follows the current wrapper behavior and
[RES-01 §4](RES-01-fornax-inventory.md#4-implications-for-fornax). Adobe DNG
Converter availability, redistribution, and platform obligations are not
established by the repository and require a separate release review.

### 2.7 darktable/rawspeed boundary

`[recommendation]` Do not adapt the darktable application pipeline as a raw
decoder. Darktable uses rawspeed to decode and populate RAW metadata, then
applies `rawprepare`, white balance, and later modules
([RES-04 §2.1–§2.3](RES-04-darktable.md#21-current-raw-pipeline-order)). Its
normal output is processed imagery, so wrapping that output would violate the
decode/preprocess/demosaic boundary and make double correction undetectable.

`[recommendation]` If darktable is required as a backend, the adapter boundary
must target an isolated rawspeed decoder API and return its stored sensor buffer
plus original metadata before `rawprepare`. The split reports do not establish
that stable decoder-only boundary, so this remains an experimental adapter
gate rather than an initial recommendation. darktable and rawspeed remaining
design-reference value is for preprocessing formulas, CFA crop-phase handling,
and test oracles. darktable itself is GPL-3.0, and the rawspeed integration and
linking terms require a separate license decision
([RES-04 §2.7–§2.8](RES-04-darktable.md#27-relationship-to-libraw-and-dng)).

### 2.8 Demosaicing boundary

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

### 2.9 License and maintenance decisions

`[recommendation]` Keep all upstream use at adapter boundaries:

- `fornax-core` must not expose LibRaw, rawloader, rawler, image-tiff,
  `demosaic`, or MeraRAW types.
- LibRaw can be used under LGPL-2.1 or CDDL-1.0; the chosen linking and
  distribution terms need an explicit release decision.
- The rawler optional backend carries an LGPL-2.1 review before release
  packaging. Selecting it does not authorize copying its code or deciding
  linking terms; the adapter feature must retain a license and notice decision
  in design review.
- The rawloader design reference remains LGPL-2.1. Any move from reference to
  code reuse requires the same review.
- darktable is GPL-3.0 and is not recommended as an application-level backend.
  Any rawspeed adapter requires its own license and linking review.
- `dnc` must treat Adobe DNG Converter as an external runtime dependency and
  record its distribution and platform constraints rather than treating the
  executable as part of this repository.
- image-tiff (MIT) and `demosaic` (MIT OR Apache-2.0) require retention of their
  license and notice terms. MeraRAW is Apache-2.0; using its code rather than
  its interface ideas would also require checking NOTICE obligations.
- Pin exact versions and lockfile revisions. None of the split reports found a
  published LTS policy, and rawler is pre-1.0.

The license facts and maintenance evidence are summarized in
[RES-02 §2.1](RES-02-libraw.md#21-source-and-licensing),
[RES-04 §2.8](RES-04-darktable.md#28-licensing),
[RES-06 §2.2](RES-06-rust-raw-decoders.md#22-maintenance-and-public-surface),
[RES-07 §2.1–§2.2](RES-07-rust-dng-tiff.md#21-image-rsimage-tiff), and
[RES-08 §2.1–§2.5](RES-08-rust-demosaicing.md#21-demosaic-crate-api-and-bayer-contract).

## 3. Evidence

This report intentionally cites split-report sections rather than repeating
upstream file and line evidence. The `dnc` classification additionally cites
the local wrapper because that bridge was not part of the Rust decoder survey.

| Conclusion | Supporting split-report evidence |
| --- | --- |
| LibRaw exposes unpacked native samples and rich metadata without requiring its processed-image path | [RES-02 §2.2–§2.8](RES-02-libraw.md#22-decode-lifecycle-and-buffer-meaning); [RES-01 §2.5](RES-01-fornax-inventory.md#25-libraw-wrapper) |
| The current LibRaw wrapper is adaptable but not a complete decoder boundary | [RES-01 §2.5 and §4](RES-01-fornax-inventory.md#25-libraw-wrapper); [RES-02 §2.7 and §4](RES-02-libraw.md#27-metadata-retention-and-the-safe-wrapper) |
| rawler has the broadest target-format dispatch, including CR3 | [RES-06 §2.1](RES-06-rust-raw-decoders.md#21-target-format-evidence) |
| rawler's metadata is rich but its mutation and panic surface needs an adapter | [RES-06 §2.4](RES-06-rust-raw-decoders.md#24-rawler); [RES-07 §2.2](RES-07-rust-dng-tiff.md#22-rawlers-dng-facing-pieces) |
| rawler decodes without normalization until the mutating scaling method is called | [RES-03 §2.1](RES-03-dnglab.md#21-decode-representation-and-sample-domain) |
| rawloader lacks CR3 and sufficient typed metadata for the initial backend | [RES-06 §2.3](RES-06-rust-raw-decoders.md#23-rawloader) |
| image-tiff has suitable generic TIFF plumbing but no DNG semantics or Bayer DNG fixture proof | [RES-07 §2.1](RES-07-rust-dng-tiff.md#21-image-rsimage-tiff) |
| image-tiff versus rawler versus a minimal parser has the stated tradeoffs | [RES-07 §2.3](RES-07-rust-dng-tiff.md#23-dependency-versus-minimal-parser) |
| darktable applies `rawprepare` and later processing after decoding | [RES-04 §2.1–§2.3](RES-04-darktable.md#21-current-raw-pipeline-order) |
| darktable is GPL-3.0 and is not established as a decoder-only library | [RES-04 §2.7–§2.8](RES-04-darktable.md#27-relationship-to-libraw-and-dng) |
| the `demosaic` crate has a usable Bayer contract and typed primary errors, with panic-prone conversion helpers | [RES-08 §2.1–§2.2](RES-08-rust-demosaicing.md#21-demosaic-crate-api-and-bayer-contract) |
| MeraRAW offers useful interface/test ideas but brings a broader pipeline and unresolved allocation behavior | [RES-08 §2.3–§2.5](RES-08-rust-demosaicing.md#23-meraraw-demosaic-api-and-validation) |
| existing decoder types must remain behind adapters | [RES-01 §4](RES-01-fornax-inventory.md#4-implications-for-fornax); [RES-06 §4](RES-06-rust-raw-decoders.md#4-implications-for-fornax); [RES-07 §4](RES-07-rust-dng-tiff.md#4-implications-for-fornax); [RES-08 §4](RES-08-rust-demosaicing.md#4-implications-for-fornax) |

Local `dnc` evidence:

| Conclusion | Source |
| --- | --- |
| dnc invokes an external Adobe DNG Converter executable | `crates/dnc/src/wrapper.rs:5-14`; `crates/dnc/src/wrapper.rs:66-80` |
| dnc returns a converted DNG path rather than sensor samples | `crates/dnc/src/wrapper.rs:40-82` |
| dnc is not compiled on Linux | `crates/dnc/src/lib.rs:5-8` |
| linear-DNG conversion is an available option and must not be selected by the initial backend | `crates/dnc/src/params.rs:126-140` |

The dependency gate used here is the merged aggregate PR
[#534](https://github.com/Glatzel/fornax/pull/534), whose merge commit contains
the RES-01 through RES-08 split reports and the RES-09 manifest. The RES-09
contract remains `agent-work/tasks/raw-pipeline/RES-09.toml`.

## 4. Implications for fornax

`[recommendation]` The proposed component map is:

| `fornax` boundary | Selected ecosystem role | Required isolation |
| --- | --- | --- |
| Decode contract and universal RAW model in `fornax-core` | First-party boundary, no format decoder | Backend-neutral samples, metadata, coordinates, processing state, capabilities, selection, and provenance |
| LibRaw backend adapter | Preferred existing C/C++ decoder | Reuse FFI ownership; call `unpack`; snapshot metadata; no `dcraw_process`; no LibRaw types in core |
| dnglab/rawler backend adapter | Preferred pure Rust decoder | Feature-gated, pinned, copied output; no `apply_scaling`; no rawler types in core |
| `dnc` conversion bridge | Optional Adobe-DNG normalization path | Convert to mosaic DNG and delegate to LibRaw or dnglab; explicit platform, provenance, license, and temporary-file policy |
| darktable/rawspeed backend | Experimental decoder-only adapter or design reference | Never wrap the darktable application output; isolate rawspeed, preserve pre-`rawprepare` samples, and resolve licensing |
| Demosaicing interface and implementations | `demosaic` adapter dependency | Owned Bayer/input/output/error types; validated call into upstream algorithms |
| Registry/test design | MeraRAW design reference | Reuse ideas and test patterns, not its pipeline types or application order |

This keeps decoder maintenance outside `fornax` while preserving one owned
normalization boundary. The design needs a backend registry and capability
metadata, not a first-party TIFF/RAW parser. Preprocessing remains a separate
`fornax` stage, so selecting LibRaw, dnglab/rawler, dnc, or a future rawspeed
adapter does not change where black correction or demosaicing occurs.

`[recommendation]` Start implementation planning with two adapters rather than
one backend: LibRaw as the broad existing decoder and dnglab/rawler as the pure
Rust alternative. This makes backend selection real, provides a comparison
oracle for metadata translation, and avoids making the entire decode layer
depend on either LGPL FFI or a pre-1.0 crate. The dnc bridge follows only after
its external-runtime and license policy is resolved. The darktable/rawspeed
adapter follows only after a decoder-only boundary is proven.

`[recommendation]` Support claims remain fixture-driven. Source dispatch is enough
to describe backend capability, not to state that DNG, ARW, NEF, CR2/CR3/CRW
decode correctly or preserve metadata for every camera. Each backend must run
the same target fixture matrix, report sample-domain differences explicitly,
and pass shared synthetic preprocessing/demosaicing tests after conversion to
the universal model. Until then, these are integration recommendations, not
release-support claims.

## 5. Open questions and limitations

- No inspected LibRaw, rawler, rawloader, or image-tiff checkout included target
  RAW fixtures. Impact: target-format acceptance cannot be inferred from source
  coverage or camera tables; a cross-backend fixture matrix remains mandatory.
- The current LibRaw wrapper omits black/white/crop metadata, ignores
  `raw_pitch`, and contains a four-channel pointer defect. Impact: the adapter
  requires implementation and tests before it can be the baseline backend.
- LibRaw's decoded `raw_image` meaning can vary by vendor decoder. Impact: the
  capability record and processing-state metadata must describe any
  backend-specific transformation rather than asserting one universal sample
  encoding.
- rawler's complete error, panic, and opcode surface has not been audited.
  Impact: the adapter must contain known calls and add malformed-input
  regressions before enabling the backend.
- rawler is pre-1.0 and no inspected candidate has a published long-term
  support policy. Impact: pin versions and keep adapter boundaries small enough
  to replace or patch a backend.
- The exact external-runtime, redistribution, and temporary-file behavior for
  Adobe DNG Converter is unresolved. Impact: dnc cannot be a default backend
  and cannot be considered portable until its platform and release policy is
  documented.
- A stable decoder-only API for darktable/rawspeed was not established. Impact:
  darktable remains a design reference or test oracle; its application pipeline
  cannot be used as a decode backend.
- Different backends may disagree on black defaults, crop coordinates, CFA
  phase, and saturation semantics. Impact: selection must preserve provenance,
  and the fixture suite must compare normalized metadata rather than bytes alone.
- Reusing decoders avoids maintaining format parsers but does not remove the
  need to maintain adapter code, capability declarations, license notices, and
  malformed-input tests.
- Comparative image quality was not independently reproduced for either Rust
  demosaicing implementation. Impact: `demosaic` is a code candidate, while
  the algorithm set and quality acceptance remain `RES-05`/design decisions.
- MeraRAW currently reports multiple registered strategies as stubs and does
  not reliably surface allocation failure despite an `Alloc` error variant.
  Impact: registry names cannot be treated as implemented capabilities, and
  allocation behavior must not be inherited.
- The exact LGPL-2.1 packaging and notice decisions for LibRaw and rawler remain
  release-review questions. Impact: an adapter recommendation does not remove
  the need for a license decision before distribution.
- CFA phase, active/default crop, and black-level translation depend on
  synthesis and design decisions not owned by RES-09. Impact: these mappings
  are requirements on the adapters, not finalized APIs in this report.

## 6. References

Split reports:

- [RES-01 — Existing fornax components](RES-01-fornax-inventory.md)
- [RES-02 — LibRaw](RES-02-libraw.md)
- [RES-03 — dnglab and DNG metadata](RES-03-dnglab.md)
- [RES-04 — darktable and RAW preprocessing](RES-04-darktable.md)
- [RES-06 — Rust RAW decoder crates](RES-06-rust-raw-decoders.md)
- [RES-07 — Rust TIFF and DNG building blocks](RES-07-rust-dng-tiff.md)
- [RES-08 — Rust demosaicing crates](RES-08-rust-demosaicing.md)
- [RES-09 task manifest](../tasks/raw-pipeline/RES-09.toml)

Upstream projects at the revisions compared above:

- LibRaw: <https://github.com/LibRaw/LibRaw/tree/b93f6e45c194f5df9b02a43b1af9a54b4f41f33f>
- darktable: <https://github.com/darktable-org/darktable/tree/6556cd7dbb0e69f95d165dda8b69e7270c327f6d>
- rawloader: <https://github.com/pedrocr/rawloader/tree/8f2ace2d27c7fa6e7671a7fd0ef11d1b1bd77ddf>
- rawler / dnglab: <https://github.com/dnglab/dnglab/tree/62b77de1a206d467e910d55199191068b45a9cc1>
- image-tiff: <https://github.com/image-rs/image-tiff/tree/3302936b08e2260d4c66f40f51170efbb1fded4c>
- `demosaic`: <https://github.com/naorunaoru/demosaic/tree/5050131e998b7f6414eef02eeb60f74b99d77451>
- MeraRAW Demosaic: <https://github.com/kaimai-apex/meraraw-demosaic/tree/b94ebd4324ce56218e42989a5ed852b32a52003f>
- local `dnc` wrapper: `crates/dnc/`

Source paths, symbols, line ranges, and license-file evidence remain in the
corresponding split report or local-source row above and are not duplicated here.
