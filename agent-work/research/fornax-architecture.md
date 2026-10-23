# Existing `fornax` Architecture

> **Task:** `RES-01`
> **Owner:** unassigned
> **Status:** `ready` (research not started)
> **Last updated:** 2026-10-23
> **Upstream versions:** record the local repository commit inspected
>
> **Acceptance criteria (authoritative version in the task tracker):**
>
> - [ ] Every public trait, type, and function relevant to decoding, metadata,
>   preprocessing, or demosaicing is listed with path and symbol.
> - [ ] Current limitations are explicit, including missing metadata,
>   black/white levels, crop, and non-Bayer support.
> - [ ] Reuse candidates are separated from code that must be replaced.
> - [ ] Dependency direction is documented.
> - [ ] Findings use `[verified]`, `[inference]`, or `[recommendation]` labels.
> - [ ] All six required report sections are complete.

## 1. Scope and questions

Audit the existing repository architecture, crate boundaries, data structures,
dependencies, and reusable implementations. Questions to answer:

- What do `IDecoder`, `IPostProcessor`, `BayerImage`, `BayerPattern`,
  `IDemosaic`, and `DemosaicLinear` actually guarantee?
- Which public APIs are stable constraints and which are incidental?
- How do dependencies flow between `fornax-core`, `fornax-dalim`, `libraw`,
  `libraw-sys`, `fornax`, `fornax-py`, `dnc`, and `fornax-devtool`?
- Which existing code can be reused for the universal representation,
  preprocessing, or demosaicing?
- Which current assumptions block non-Bayer, metadata-rich, or black-level-aware
  processing?

_No findings recorded yet._

## 2. Findings

Not started. Record only source-verified behavior here.

## 3. Evidence

Not started. Cite local file paths, line ranges, symbols, Cargo manifests, CI
files, and test names.

## 4. Implications for fornax

To be derived from the findings. State which existing abstractions can be
extended, which must be replaced, and which crate boundaries must change.

## 5. Open questions and limitations

Record unresolved questions, unverified assumptions, and missing evidence.

## 6. References

- Local repository paths and symbols: not yet recorded.
- CI and tooling files inspected: not yet recorded.
- Relevant tests and benchmarks: not yet recorded.
