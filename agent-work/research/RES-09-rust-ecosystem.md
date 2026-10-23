# Rust Ecosystem Comparison and Reuse Recommendation

> **Task:** `RES-09`
> **Owner:** unassigned
> **Status:** `blocked` on user approval of `RES-06`, `RES-07`, and `RES-08`
> **Last updated:** 2026-10-23
>
> **Acceptance criteria (authoritative version in the task tracker):**
>
> - [ ] Every candidate is compared on scope, Bayer/target-format fit, API
>   boundary, maintenance, and license.
> - [ ] Every recommendation links to a split report.
> - [ ] Dependency versus optional-backend versus design-reference status is
>   explicit.
> - [ ] No upstream RAW type is recommended for `fornax-core` without an adapter
>   and license decision.
> - [ ] Remaining unknowns and their impact are listed.
> - [ ] The report contains the six required sections and cites the split
>   reports.

## 1. Scope and questions

This report will compare, rather than re-research, the Rust ecosystem evidence
owned by:

- `agent-work/research/RES-06-rust-raw-decoders.md` (`RES-06`);
- `agent-work/research/RES-07-rust-dng-tiff.md` (`RES-07`); and
- `agent-work/research/RES-08-rust-demosaicing.md` (`RES-08`).

The comparison will answer:

- Which components are candidates for a dependency, an optional adapter, a
  design reference, or no reuse?
- Does each candidate preserve the initial conventional Bayer and
  DNG/ARW/NEF/CR* scope without forcing deferred sensor layouts into the core?
- Where are the API, maintenance, and licensing boundaries?
- What must be verified before implementation?

## 2. Findings

No comparison is finalized. The `RES-09` worker must read the three source
reports after their acceptance review and record a decision matrix here.

The initial candidate buckets are:

| Bucket | Sources to compare |
| --- | --- |
| RAW decoder adapters | rawloader and rawler from `RES-06` |
| DNG/TIFF building blocks | image-tiff and DNG-facing rawler pieces from `RES-07` |
| Demosaicing implementations and API patterns | `demosaic` and MeraRAW from `RES-08` |

## 3. Evidence

This report will cite sections in `RES-06`–`RES-08` rather than duplicate source
paths, commits, and line ranges. Source-level evidence remains owned by the split
reports.

## 4. Implications for fornax

The recommendation is intentionally pending. A valid finished comparison must
cover:

- `fornax-core` independence from upstream decoder types;
- the adapter boundary for any optional decoder backend;
- the DNG/TIFF dependency or minimal-parser decision;
- independent Bayer demosaicing strategy support;
- LGPL, MIT, and Apache-2.0 obligations; and
- the explicit rejection/deferral boundary for X-Trans, Foveon, Quad Bayer,
  four-color, monochrome, non-mosaic payloads, and other RAW formats.

## 5. Open questions and limitations

- `RES-06`–`RES-08` are not yet accepted.
- Target-format fixture coverage is unknown in the split reports.
- No dependency or license decision is final.
- Algorithm quality and final interface shape remain downstream of `RES-05`,
  synthesis, and design.

## 6. References

- [`RES-06-rust-raw-decoders.md`](RES-06-rust-raw-decoders.md) — `RES-06`.
- [`RES-07-rust-dng-tiff.md`](RES-07-rust-dng-tiff.md) — `RES-07`.
- [`RES-08-rust-demosaicing.md`](RES-08-rust-demosaicing.md) — `RES-08`.
- [`RES-05-demosaicing.md`](RES-05-demosaicing.md) — algorithm scope from `RES-05`.
