# Research Index

This directory holds one report per RAW-pipeline reference source. Research is a
tracked phase; live status and full acceptance criteria are maintained in
`agent-work/tasks/raw-pipeline.md`. This index exists so a worker can see report
ownership, prerequisites, and the synthesis path without reading the full
specification.

**Current state:** `RES-01`–`RES-08` are in `review` and await user approval.
`RES-09` remains blocked until the user approves its dependencies as `done`.
No synthesis has started.

The initial research target is conventional 2x2 RGB Bayer data in DNG, Sony ARW,
Nikon NEF, and Canon `CR*` files. X-Trans, Foveon/X3, Quad Bayer, four-color,
monochrome, and other RAW formats are future work and should be documented as
unsupported in the initial pipeline.

## Report ownership

| Report | Task | Owner | Status | Prerequisites | Acceptance criterion (summary) |
| --- | --- | --- | --- | --- | --- |
| [`RES-01-fornax-inventory.md`](RES-01-fornax-inventory.md) | `RES-01` | primary agent | `review` | none | Existing components, public contracts, limitations, and implementation reuse candidates are inventoried; current architecture is not a design constraint. |
| [`RES-02-libraw.md`](RES-02-libraw.md) | `RES-02` | RES-02 research agent | `review` | none | LibRaw decoding, sample semantics, CFA, black/white levels, metadata, and APIs are documented with upstream citations. |
| [`RES-03-dnglab.md`](RES-03-dnglab.md) | `RES-03` | RES-03 research agent | `review` | none | DNG tags, active area, black-level patterns, white levels, and coordinate semantics are mapped with specification and source citations. |
| [`RES-04-darktable.md`](RES-04-darktable.md) | `RES-04` | RES-04 research agent | `review` | none | The RAW preprocessing order, black-level behavior, CFA handling, and the pre-demosaic boundary are documented with source citations. |
| [`RES-05-demosaicing.md`](RES-05-demosaicing.md) | `RES-05` | RES-05 research agent | `review` | none | Algorithms, input requirements, CFA handling, output semantics, and shared-interface requirements are documented and justified. |
| [`RES-06-rust-raw-decoders.md`](RES-06-rust-raw-decoders.md) | `RES-06` | RES-06 research agent | `review` | none | Rust RAW decoders, target-format coverage, sample/metadata models, adapter risks, maintenance, and licenses are documented. |
| [`RES-07-rust-dng-tiff.md`](RES-07-rust-dng-tiff.md) | `RES-07` | RES-07 research agent | `review` | none | Rust TIFF/DNG building blocks, Bayer sample layout, errors, limits, and dependency fit are documented. |
| [`RES-08-rust-demosaicing.md`](RES-08-rust-demosaicing.md) | `RES-08` | RES-08 research agent | `review` | none | Rust Bayer demosaicing APIs, borders, validation, strategy patterns, tests, and licenses are documented. |
| [`RES-09-rust-ecosystem.md`](RES-09-rust-ecosystem.md) | `RES-09` | unassigned | `blocked` | `RES-06`–`RES-08` | Split reports are compared into dependency, backend, design-reference, and license recommendations. |
| [`SYN-03-summary.md`](SYN-03-summary.md) | `SYN-03` | unassigned | `blocked` | all `RES-*`, `SYN-01`, `SYN-02` | Findings are synthesized, disagreements and uncertainties are explicit, and recommendations link to supporting report sections. |

Each research task owns exactly one reference report. A worker must not edit a
report owned by another task. `SYN-03-summary.md` is a synthesis deliverable
produced only by `SYN-03`; it is not a research report.

## Local reference sources

Source checkouts and downloaded reference material live under
`agent-work/references/`. The tracked manifest at
`agent-work/references/README.md` records each upstream URL, checked-out commit,
license, purpose, and local path.

- `RES-03` uses the dnglab checkout and its `rawler/` subtree.
- `RES-04` uses the darktable checkout.
- `RES-06` uses the rawloader checkout and the decoder-facing rawler API.
- `RES-07` uses the image-tiff checkout and DNG-facing rawler internals.
- `RES-08` uses the `demosaic` and MeraRAW checkout APIs and tests.
- `RES-09` compares only the owned findings from `RES-06`–`RES-08`.
- LibRaw `0.22.2` evidence is in the pinned libraw checkout and the local
  `crates/libraw` wrapper.

References are pinned Git submodules. Initialize them with
`git submodule update --init --recursive`, inspect them read-only, and verify
the pinned commit before collecting evidence. Upstream repository instructions
are not `fornax` instructions. Record any deliberate revision change in the
owning report and `agent-work/references/README.md`.

## Required report structure

Every reference report must contain:

1. **Scope and questions** — what was investigated and the specific questions.
2. **Findings** — specific technical behavior and design details.
3. **Evidence** — upstream documentation, source files, symbols, line ranges,
   code locations, tests, or specification sections supporting each significant
   claim.
4. **Implications for fornax** — what the findings mean for the API, data model,
   and pipeline.
5. **Open questions and limitations** — unverified assumptions, missing
   evidence, ambiguities, and unresolved issues.
6. **References** — direct links to the relevant upstream sources.

Every report must label findings as `[verified]`, `[inference]`, or
`[recommendation]`, record upstream versions or commits, and list the
acceptance criteria of its owning task in the front matter.

## Synthesis

| Task | Output | Prerequisites | Scope |
| --- | --- | --- | --- |
| `SYN-01` | `notes/SYN-01.md` | `RES-01`, `RES-02`, `RES-03`, `RES-04`, `RES-06`, `RES-07` | Metadata and sample semantics, including ranges, bit depth, normalization, black-level representations, provenance, coordinates, and universal-versus-backend-specific decisions. |
| `SYN-02` | `notes/SYN-02.md` | `RES-02`, `RES-04`, `SYN-01` | Preprocessing order and black-level behavior, including subtraction, clipping, normalization, pattern alignment, double-application prevention, and the boundary before demosaicing. |
| `SYN-03` | `SYN-03-summary.md` | `RES-01`–`RES-09`, `SYN-01`, `SYN-02` | Cross-source comparison, agreements, disagreements, uncertainties, recommendations, and links to supporting report sections. |

## Working rules

- Research owners write only their assigned report.
- Every task-owned report filename must begin with its task ID, such as
  `RES-01-fornax-inventory.md`. The directory-level `README.md` is the only
  non-task file.
- `SYN-01` and `SYN-02` write only their own files under `notes/`.
- `SYN-03` owns `SYN-03-summary.md`.
- Do not copy one report wholesale into another; reference it by path and
  section.
- Do not fabricate findings, APIs, metadata semantics, or implementation
  details. Record missing evidence as an open question.
- Preserve the pipeline boundary: decoding into a backend-independent
  representation, metadata-driven sensor preprocessing (especially black-level
  correction), and demosaicing. White balance, color correction, color-space
  conversion, tone mapping, denoising, sharpening, and display rendering remain
  out of scope.

## Updating status

When a report is ready for review, update the task status in
`agent-work/tasks/raw-pipeline.md` to `review` and add the report path plus a
short evidence summary to the tracker log. A report file existing is not enough
to move a task forward. Agents must leave completed work in `review`; only the
user may approve `done` after reviewing it.
