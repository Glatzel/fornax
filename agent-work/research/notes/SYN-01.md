# SYN-01 — Metadata and Sample Semantics Reconciliation

> **Task:** `SYN-01`
> **Status:** `blocked` (prerequisite reports not accepted)
> **Owner:** unassigned
> **Last updated:** 2026-10-23
>
> **Prerequisites:** `RES-01`, `RES-02`, `RES-03`, `RES-04`, `RES-06`, `RES-07`
>
> **Output consumed by:** `SYN-02`, `SYN-03`, design tasks `DES-001`–`DES-005`.

## Scope

Reconcile, across the accepted reports:

- Stored versus black-corrected sample values.
- Numeric ranges, bit depth, normalization, and white-level semantics.
- Black-level representations as metadata: scalar, per-channel, and spatially
  varying patterns.
- Metadata units, provenance, optionality, and conflicts between sources.
- Sensor coordinates, active-area coordinates, cropped-image coordinates, and
  CFA alignment.
- Universal versus backend-specific concepts.

## Decision table

| Item | Chosen meaning | Unit | Coordinate convention | Valid range | Supporting report sections | Authority rule | Remaining uncertainty |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Not started | | | | | | | |

## Conflict log

For each disagreement, record the conflicting evidence and the chosen
resolution or explicit deferral. Do not silently reconcile.

| Conflict | Source A | Source B | Resolution | Status |
| --- | --- | --- | --- | --- |
| Not started | | | | |

## Open questions

- Not started.

## Acceptance checklist

- [ ] Decision table complete for every scoped item.
- [ ] Stored versus black-corrected values unambiguous.
- [ ] Bit depth, normalization, and white-level semantics stated.
- [ ] Scalar, per-channel, and spatially varying black-level representations
  classified.
- [ ] Metadata units, provenance, optionality, and conflicts resolved or left
  explicitly open.
- [ ] Coordinate spaces and CFA alignment defined.
- [ ] Universal versus backend-specific concepts separated.
- [ ] Every conclusion links to a supporting report section.
