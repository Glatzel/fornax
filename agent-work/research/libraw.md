# LibRaw

> **Task:** `RES-02`
> **Owner:** unassigned
> **Status:** `ready` (research not started)
> **Last updated:** 2026-10-23
> **Upstream versions:** record the LibRaw version or commit inspected
>
> **Acceptance criteria (authoritative version in the task tracker):**
>
> - [ ] Relevant fields are documented with type, unit, coordinate convention,
>   sentinel values, and validity conditions.
> - [ ] Black-level repeat-pattern and per-channel semantics are explained,
>   including sensor-coordinate alignment.
> - [ ] `raw_image` versus processed output is distinguished.
> - [ ] A field-to-universal-candidate mapping is provided.
> - [ ] Unverified claims are labeled `[inference]`.
> - [ ] All six required report sections are complete.

## 1. Scope and questions

Investigate LibRaw decoding behavior, sample semantics, CFA representation,
black/white levels, metadata, and relevant APIs. Questions to answer:

- How is a RAW file parsed, unpacked, and exposed?
- Does a decoded buffer contain original sensor codes, unpacked samples,
  linearized samples, or processed data?
- How are dimensions, stride, sample type, CFA pattern, and CFA origin
  represented?
- How are black level, white level, saturation, masked pixels, and spatially
  varying levels represented and interpreted?
- How are active area, crop, margins, and orientation handled?
- Which metadata is retained, normalized, derived, or discarded?
- What does the existing `libraw` Rust wrapper expose and hide?

_No findings recorded yet._

## 2. Findings

Not started. Record only source-verified behavior here.

## 3. Evidence

Not started. Cite upstream repository URL, version or commit, source path,
symbol, line range, tests, and documentation.

## 4. Implications for fornax

To be derived from the findings. Cover sample semantics, metadata mapping,
CFA origin, black-level representation, and decoder-adapter requirements.

## 5. Open questions and limitations

Record unresolved questions, unverified assumptions, and missing evidence.

## 6. References

- Upstream repository and commit: not yet recorded.
- Source files and symbols: not yet recorded.
- Tests and documentation: not yet recorded.
- License constraints: not yet recorded.
