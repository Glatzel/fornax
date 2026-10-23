# dnglab and DNG Metadata

> **Task:** `RES-03`
> **Owner:** unassigned
> **Status:** `ready` (research not started)
> **Last updated:** 2026-10-23
> **Upstream versions:** record the dnglab version or commit and the DNG
> specification revision inspected
>
> **Acceptance criteria (authoritative version in the task tracker):**
>
> - [ ] Relevant DNG tags are mapped to source symbols with units, defaults, and
>   coordinate conventions.
> - [ ] Active area, black-level patterns, white levels, and orientation are
>   covered or explicitly excluded with a reason.
> - [ ] DNG versus vendor RAW metadata differences are identified.
> - [ ] Licensing constraints are recorded.
> - [ ] All six required report sections are complete.

## 1. Scope and questions

Investigate dnglab and DNG metadata interpretation. Questions to answer:

- How does dnglab parse DNG files and expose the raw image?
- What do `BlackLevel`, `WhiteLevel`, `BlackLevelRepeatDim`, `CFAPattern`,
  `CFARepeatPatternDim`, `ActiveArea`, `DefaultCrop*`, and `Orientation` mean,
  including units, defaults, and coordinate conventions?
- How do active-area and cropped-image coordinates relate to the stored buffer?
- How are black-level patterns aligned with CFA phase?
- Which tags does dnglab normalize, preserve, or discard?
- Where does the DNG specification leave behavior ambiguous?
- What licensing constraints apply to any considered reuse?

_No findings recorded yet._

## 2. Findings

Not started. Record only source-verified behavior here.

## 3. Evidence

Not started. Cite dnglab repository URL, version or commit, source paths,
symbols, tests, and the relevant DNG specification sections.

## 4. Implications for fornax

To be derived from the findings. Cover tag mapping, coordinate semantics,
active-area computation, black-level patterns, and white-level normalization.

## 5. Open questions and limitations

Record unresolved questions, unverified assumptions, and missing evidence.

## 6. References

- Upstream repository and commit: not yet recorded.
- Source files and symbols: not yet recorded.
- DNG specification sections: not yet recorded.
- Tests: not yet recorded.
- License constraints: not yet recorded.
