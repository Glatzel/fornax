# SYN-02 — Preprocessing Order and Black-Level Behavior

> **Task:** `SYN-02`
> **Status:** `blocked` (prerequisite reports and `SYN-01` not accepted)
> **Owner:** unassigned
> **Last updated:** 2026-10-23
>
> **Prerequisites:** `RES-02`, `RES-04`, `SYN-01`
>
> **Output consumed by:** `SYN-03`, design tasks `DES-001`, `DES-004`, `DES-005`.

## Scope

Reconcile, across the accepted reports and `SYN-01`:

- The order and semantics of sensor-domain preprocessing.
- Black-level selection, subtraction, clipping, underflow, saturation,
  normalization, and rounding.
- Pattern alignment with crop, margins, and CFA phase.
- Prevention of double application.
- The precise boundary before demosaicing and which later operations must
  remain unapplied.
- Mandatory, optional, format-dependent, and unsupported operations.

## Operation table

| Order | Operation | Input domain | Output domain | Required metadata | Formula / rule | Error behavior | Test strategy |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Not started | | | | | | | |

## Boundary statement

State where sensor-domain preprocessing ends, what the demosaicing interface
receives, and which later operations are excluded.

_Not started._

## Open questions

- Not started.

## Acceptance checklist

- [ ] Operation table complete with inputs, outputs, formulas, ordering, and
  test strategy.
- [ ] Black-level representation and CFA-aware selection defined.
- [ ] Subtraction, clipping, underflow, saturation, normalization, and rounding
  specified numerically.
- [ ] Boundary before demosaicing explicit; later operations named as excluded.
- [ ] Missing, invalid, and inconsistent metadata behavior specified.
- [ ] Processing-state semantics prevent double correction.
- [ ] Every decision links to a supporting report or `SYN-01` item.
