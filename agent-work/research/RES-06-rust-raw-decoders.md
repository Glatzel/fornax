# Rust RAW Decoder Crates

> **Task:** `RES-06`
> **Owner:** RES-06 research agent
> **Review:** GitHub task pull request; its open/merged state is authoritative.
> **Last updated:** 2026-10-23
> **Upstream versions:** rawloader
> `8f2ace2d27c7fa6e7671a7fd0ef11d1b1bd77ddf`; rawler as part of dnglab
> `62b77de1a206d467e910d55199191068b45a9cc1`.
>
> **Acceptance criteria (authoritative version in the task TOML manifest):**
>
> - [x] Each decoder has a source-verified sample and metadata model.
> - [x] Coverage of DNG, Sony ARW, Nikon NEF, and Canon `CR*` is explicit.
> - [x] Conventional Bayer behavior and non-Bayer handling are explicit.
> - [x] Reusable code and ideas are distinguished with license constraints.
> - [x] Maintenance and adapter risks are supported by evidence.
> - [x] Inferred findings are labeled `[inference]`.
> - [x] The report contains the six required sections and cites sources.
>
> **Verification snapshot:** RES-06 research agent and primary checks completed
> 2026-10-23; GitHub pull-request review is pending. Decoder dispatch, public representations,
> errors, preprocessing state, target-format coverage, Bayer/non-Bayer
> behavior, maintenance, and licenses were checked against the pinned
> rawloader and rawler sources. Coverage is source-verified only; no target RAW
> fixtures were available for decode execution.

## 1. Scope and questions

This report evaluates Rust RAW decoder crates as optional backends or as design
evidence for the conventional Bayer pipeline. It does not repeat LibRaw
internals (`RES-02`), DNG tag semantics (`RES-03`), TIFF parsing internals
(`RES-07`), or demosaicing APIs (`RES-08`).

The questions were:

- What does each decoder expose at its public boundary?
- Does it preserve stored sensor codes, crop/CFA alignment, and preprocessing
  state separately?
- Which initial target formats are verified for the exact inspected revision?
- How does each decoder handle non-Bayer payloads that `fornax` defers?
- What can be reused as code versus only as an API or test idea?

## 2. Findings

### 2.1 Target-format evidence

| Decoder | DNG | Sony ARW | Nikon NEF | Canon `CR*` | Evidence level |
| --- | --- | --- | --- | --- | --- |
| rawloader 0.37.2 | Dispatch by `DNGVersion` | `SONY` → `ArwDecoder` | `NIKON CORPORATION` → `NefDecoder` | `CR2` and `CRW` dispatch paths; no `CR3` module or BMFF path | Source dispatch verified; no fixture run |
| rawler 0.8.0 | Dispatch by `DNGVersion` | `SONY` → `ArwDecoder` | `Nikon`/`NIKON CORPORATION` → `NefDecoder` | CIFF → `CRW`; Canon TIFF → `CR2`; BMFF `crx` → `CR3` with default `crx` feature | Source dispatch verified; no fixture run |

Sources: rawloader `src/decoders/mod.rs:410-475`; rawler
`src/decoders/mod.rs:143,1017-1049,1067-1109`.

`[verified]` rawloader has decoder modules for DNG, ARW, NEF, CR2, and CRW, but
no CR3/BMFF module in its source tree. Its target Canon coverage therefore does
not include CR3 at this revision. rawler has dedicated CR3/CRX decoding and
includes it in its supported-extension list. `[inference]` rawler is the
stronger candidate when Canon CR3 is a required acceptance format.

No DNG, ARW, NEF, CR2, CR3, or CRW fixture was present in either checkout's
searched sample paths. The table proves dispatch and source presence, not
successful camera/format decoding or correct metadata semantics.

### 2.2 Maintenance and public surface

| Decoder | Version | Latest inspected commit | License | Public entry point |
| --- | --- | --- | --- | --- |
| rawloader | 0.37.2 | `8f2ace2` (2026-08-16) | LGPL-2.1 | `decode_file` / `decode` returning `Result<RawImage, String>` |
| rawler | 0.8.0 | `62b77de` (2026-10-23) | LGPL-2.1 | `decode_file` / `decode` returning `RawImage` or typed `RawlerError` |

`[verified]` Both public decode entry points catch decoder panics and convert
them into decoder errors (`rawloader/src/decoders/mod.rs:521-530`;
`rawler/src/decoders/mod.rs:1181-1187`). This does not make internal APIs safe:
CFA constructors and direct demosaic dispatch still contain panicking
preconditions.

### 2.3 rawloader

`[verified]` The public API is `decode_file` / `decode`, returning `RawImage`
with string errors (`src/lib.rs:78-136`). `RawImage` contains:

- make/model strings and cleaned names;
- full width, height, and components per pixel;
- four WB coefficients in RGBE order;
- four `u16` white and black levels in RGBE order;
- a 4x3 XYZ-to-camera matrix;
- a general CFA pattern;
- crop borders in top/right/bottom/left order;
- masked black-area rectangles;
- orientation; and
- `RawImageData::{Integer(Vec<u16>), Float(Vec<f32>)}`.

Source: `src/decoders/image.rs:5-51,55-112`.

`[verified]` `CFA` stores a pattern name, width, height, color count, and an
expanded lookup table. Pattern size is inferred from string length: 2x2, 6x6,
2x8, or 12x12; other lengths panic (`src/decoders/cfa.rs:21-112`). This includes
layouts outside the conventional Bayer target.

`[verified]` rawloader does not expose an explicit stride, a typed sample-domain
or processing state, arbitrary black-level patterns, an active-area type, or
structured decoder error categories. Integer and float buffers do not identify
whether values are stored codes or decoder-produced samples.

`[inference]` rawloader is useful behind an adapter or as a format-catalog
reference, but its fixed four-level metadata and string errors are insufficient
for the universal representation.

### 2.4 rawler

`[verified]` rawler's `RawImage` contains integer or float samples; dimensions,
channels, camera names, WB, white level, and rational
`BlackLevel {levels, cpp, width, height}`; CFA plus plane-color mapping;
`RawPhotometricInterpretation::{BlackIsZero, Cfa, LinearRaw}`; active and crop
areas; black areas; orientation; color matrices; preserved DNG tags; opcode
lists; a linearization table; and capture metadata
(`rawler/src/rawimage.rs:203-276`).

`[verified]` `RawImage::apply_scaling` applies linearization, converts samples
to float, applies black/white normalization, and resets black/white metadata
(`rawler/src/rawimage.rs:546-576`). This mutating method is a double-correction
hazard for a staged API.

`[verified]` rawler's demosaicing interface is separate from the RAW image model,
but unsupported CFA dispatch can panic rather than return one common algorithm
error (`rawler/src/imgop/sensor/mod.rs:29-66`;
`rawler/src/imgop/sensor/bayer/ppg.rs:35-50`).

`[inference]` rawler is the richest decoder evidence in this split, but its
monolithic mutable state and panic-prone preconditions should inform, not define,
`fornax`'s core types.

### 2.5 Reuse and licensing

| Decoder | Reusable code | Reusable ideas | Constraint |
| --- | --- | --- | --- |
| rawloader | Potentially behind an adapter | Camera catalog, CFA shift, clean camera naming | LGPL-2.1; fixed four levels; string errors; incomplete metadata |
| rawler | Potentially as an optional backend | Rational black tiles, active/default crop split, CFA plane mapping, photometric interpretation | LGPL-2.1; mutating scaling; panic-prone APIs |

`[recommendation]` Keep decoder-specific types out of `fornax-core`. Any
rawloader/rawler dependency is an adapter or optional backend decision that must
pass the architectural and license review in `RES-09`.

## 3. Evidence

| Finding | Evidence |
| --- | --- |
| rawloader public decode and `RawImage` | `rawloader/src/lib.rs:78-136`; `src/decoders/image.rs:5-112` |
| rawloader target dispatch and absent CR3 path | `rawloader/src/decoders/mod.rs:410-475`; decoder module list at `:48-110` |
| rawloader CFA limits | `rawloader/src/decoders/cfa.rs:21-112` |
| rawloader license | `rawloader/Cargo.toml`; `LICENSE` |
| rawler target dispatch | dnglab `rawler/src/decoders/mod.rs:143,1017-1049,1067-1109` |
| rawler public representation | dnglab `rawler/src/rawimage.rs:203-276` |
| rawler mutating scaling | dnglab `rawler/src/rawimage.rs:546-576` |
| rawler demosaic preconditions | dnglab `rawler/src/imgop/sensor/mod.rs:29-66`; Bayer implementations |
| rawler license | dnglab `rawler/Cargo.toml`; repository license |
| Decoder error surfaces | rawloader `src/decoders/mod.rs:521-530`; rawler `src/lib.rs:142-171`; `src/decoders/mod.rs:1181-1187` |

All paths are relative to clean pinned submodules under
`agent-work/references/`.

## 4. Implications for fornax

`[recommendation]` The decoder boundary must expose stored sample domain,
dimensions/stride, CFA layout and origin, crop/active areas, black/white
metadata, orientation, provenance, and processing state explicitly.

`[recommendation]` Do not infer target-format support from upstream camera
catalog breadth. Add fixture-based support claims for DNG, ARW, NEF, and `CR*`
only after decode and metadata validation.

`[recommendation]` X-Trans, Foveon, Quad Bayer, four-color, and monochrome paths
are deferred. A decoder adapter may encounter them, but the `fornax` boundary
must reject them with a structured unsupported-layout error.

## 5. Open questions and limitations

- The exact target-format and camera coverage of each decoder was not verified
  through real fixture decoding.
- rawloader has no CR3/CRX decode path at the pinned revision; CR3 coverage must
  come from rawler, LibRaw, or a separate implementation.
- rawler's complete error and panic surface was not audited.
- No license decision is finalized here.
- A decoder crate's successful parse does not prove CFA or black-level
  correctness; those semantics require separate synthesis and tests.

## 6. References

- rawloader: `https://github.com/pedrocr/rawloader.git` at
  `8f2ace2d27c7fa6e7671a7fd0ef11d1b1bd77ddf`; LGPL-2.1.
- rawler / dnglab: `https://github.com/dnglab/dnglab.git` at
  `62b77de1a206d467e910d55199191068b45a9cc1`; LGPL-2.1.
- Pinned checkouts: `agent-work/references/other/rawloader/` and
  `agent-work/references/dnglab/rawler/`.
