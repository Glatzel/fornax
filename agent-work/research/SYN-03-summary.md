# RAW Pipeline Research Synthesis

> **Task:** `SYN-03`
> **Owner:** unassigned
> **Status:** `blocked` (research and synthesis prerequisites not user-approved)
> **Last updated:** 2026-10-23
>
> **Prerequisites:** `RES-01`–`RES-09`, `SYN-01`, `SYN-02`
>
> **Acceptance criteria (authoritative version in the task tracker):**
>
> - [ ] Every reference report is represented and linked.
> - [ ] Agreements and disagreements are explicit, with conflicting evidence.
> - [ ] Recommendations are labeled and traceable to report sections.
> - [ ] Remaining questions and assumptions are listed with impact.
> - [ ] The summary synthesizes rather than concatenates.
> - [ ] No unreferenced claim is introduced.

## 1. Purpose and method

This report synthesizes the nine reference reports and the two reconciliation
notes. It compares findings, records disagreements, makes evidence-backed
recommendations for `fornax`, and identifies what remains unresolved. It must not
be produced until all prerequisites are user-approved as `done`.

_Not started. This file is a required structure, not a completed synthesis._

## 2. Source reports and scope

Link each report with its task ID and the sections used:

- [`RES-01-fornax-architecture.md`](RES-01-fornax-architecture.md) (`RES-01`): not yet summarized.
- [`RES-02-libraw.md`](RES-02-libraw.md) (`RES-02`): not yet summarized.
- [`RES-03-dnglab.md`](RES-03-dnglab.md) (`RES-03`): not yet summarized.
- [`RES-04-darktable.md`](RES-04-darktable.md) (`RES-04`): not yet summarized.
- [`RES-05-demosaicing.md`](RES-05-demosaicing.md) (`RES-05`): not yet summarized.
- [`RES-06-rust-raw-decoders.md`](RES-06-rust-raw-decoders.md) (`RES-06`): not yet summarized.
- [`RES-07-rust-dng-tiff.md`](RES-07-rust-dng-tiff.md) (`RES-07`): not yet summarized.
- [`RES-08-rust-demosaicing.md`](RES-08-rust-demosaicing.md) (`RES-08`): not yet summarized.
- [`RES-09-rust-ecosystem.md`](RES-09-rust-ecosystem.md) (`RES-09`): not yet summarized.
- [`notes/SYN-01.md`](notes/SYN-01.md): not yet summarized.
- [`notes/SYN-02.md`](notes/SYN-02.md): not yet summarized.

## 3. Metadata and sample semantics

Reconcile stored versus black-corrected sample values; numeric ranges, bit
depth, normalization, and white-level semantics; metadata units, provenance,
optionality, and conflicts. Cite `SYN-01` and the supporting reports.

_Not started._

## 4. Coordinates and CFA alignment

Reconcile sensor coordinates, active-area coordinates, cropped-image
coordinates, margins, orientation, and CFA phase. Cite `SYN-01` and the
supporting reports.

_Not started._

## 5. Preprocessing order and black-level behavior

Reconcile the order and semantics of sensor-domain preprocessing, including
scalar, per-channel, repeat-pattern, and spatially varying black levels,
subtraction, clipping, normalization, rounding, and double-application
prevention. Cite `SYN-02` and the supporting reports.

_Not started._

## 6. Boundary before demosaicing

State precisely where sensor-domain preprocessing ends, what the demosaicing
interface receives, and which later operations must remain unapplied. Cite
`RES-04`, `RES-05`, and `SYN-02`.

_Not started._

## 7. Universal versus backend-specific concepts

Separate concepts that can be represented by universal types from those that
must remain decoder- or format-specific. Cite `SYN-01`, `SYN-02`, and the
relevant research reports.

_Not started._

## 8. Agreements across sources

List findings on which the sources agree, with report and section links.

_Not started._

## 9. Disagreements and conflicting evidence

For each disagreement, record the conflicting evidence, the source locations,
and either a recommended resolution or an explicit deferral. Do not silently
reconcile.

_Not started._

## 10. Recommendations for fornax

Each recommendation must include:

- The recommendation and its scope.
- Supporting report sections and task IDs.
- Confidence and any assumptions.
- Impact on the data model, preprocessing API, or demosaicing interface.
- What would invalidate it.

_Not started._

## 11. Open questions and remaining uncertainty

List unresolved questions, missing evidence, ambiguities, and the design task
each one blocks.

_Not started._

## 12. References

Link every reference report and synthesis note used. Do not repeat the full
reference lists from those reports.

- Not yet assembled.
