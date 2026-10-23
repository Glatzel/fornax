# Research Index

This directory holds one report per RAW-pipeline reference source. Research is a
tracked phase; live status and full acceptance criteria are maintained in
`agent-work/tasks/raw-pipeline.md`. This index exists so a worker can see report
ownership, prerequisites, and the synthesis path without reading the full
specification.

**Current state:** research in progress. `RES-01` and `RES-02` are in `review`;
`RES-03` is `blocked` on unavailable dnglab/DNG sources. `RES-04`–`RES-06`
remain `ready`. No synthesis has started.

## Report ownership

| Report | Task | Owner | Status | Prerequisites | Acceptance criterion (summary) |
| --- | --- | --- | --- | --- | --- |
| [`fornax-architecture.md`](fornax-architecture.md) | `RES-01` | research agent | `review` | none | Existing crates, public types, dependencies, reuse candidates, and gaps are documented with local paths and symbols. |
| [`libraw.md`](libraw.md) | `RES-02` | research agent | `review` | none | LibRaw decoding, sample semantics, CFA, black/white levels, metadata, and APIs are documented with upstream citations. |
| [`dnglab.md`](dnglab.md) | `RES-03` | research agent | `blocked` | none | DNG tags, active area, black-level patterns, white levels, and coordinate semantics are mapped with specification and source citations. |
| [`darktable.md`](darktable.md) | `RES-04` | unassigned | `ready` | none | The RAW preprocessing order, black-level behavior, CFA handling, and the pre-demosaic boundary are documented with source citations. |
| [`demosaicing.md`](demosaicing.md) | `RES-05` | unassigned | `ready` | none | Algorithms, input requirements, CFA handling, output semantics, and shared-interface requirements are documented and justified. |
| [`rust-ecosystem.md`](rust-ecosystem.md) | `RES-06` | unassigned | `ready` | none | Relevant Rust projects, reusable components, API designs, dependencies, maintenance status, and licenses are documented. |
| [`summary.md`](summary.md) | `SYN-03` | unassigned | `blocked` | all `RES-*`, `SYN-01`, `SYN-02` | Findings are synthesized, disagreements and uncertainties are explicit, and recommendations link to supporting report sections. |

Each research task owns exactly one reference report. A worker must not edit a
report owned by another task. `summary.md` is a synthesis deliverable produced
only by `SYN-03`; it is not a research report.

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
| `SYN-01` | `notes/SYN-01.md` | `RES-01`, `RES-02`, `RES-03`, `RES-04`, `RES-06` | Metadata and sample semantics, including ranges, bit depth, normalization, black-level representations, provenance, coordinates, and universal-versus-backend-specific decisions. |
| `SYN-02` | `notes/SYN-02.md` | `RES-02`, `RES-04`, `SYN-01` | Preprocessing order and black-level behavior, including subtraction, clipping, normalization, pattern alignment, double-application prevention, and the boundary before demosaicing. |
| `SYN-03` | `summary.md` | all `RES-*`, `SYN-01`, `SYN-02` | Cross-source comparison, agreements, disagreements, uncertainties, recommendations, and links to supporting report sections. |

## Working rules

- Research owners write only their assigned report.
- `SYN-01` and `SYN-02` write only their own files under `notes/`.
- `SYN-03` owns `summary.md`.
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
to mark a task `done`; every acceptance criterion must be checked.
