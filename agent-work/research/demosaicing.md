# Demosaicing Algorithms and Interfaces

> **Task:** `RES-05`
> **Owner:** unassigned
> **Status:** `ready` (research not started)
> **Last updated:** 2026-10-23
> **Upstream versions:** record the versions or commits of inspected
> implementations and the exact papers cited
>
> **Acceptance criteria (authoritative version in the task tracker):**
>
> - [ ] Candidate algorithms are compared on input assumptions, output
>   semantics, border behavior, complexity, and testability.
> - [ ] A recommended initial set is justified.
> - [ ] The minimum shared input contract is stated without committing to a
>   final public API.
> - [ ] Existing `fornax-dalim` behavior and tests are documented.
> - [ ] All six required report sections are complete.

## 1. Scope and questions

Investigate demosaicing algorithms, their input requirements, CFA handling,
output semantics, and suitable common interfaces. Questions to answer:

- What input does each candidate algorithm assume: sensor codes, black-corrected
  samples, normalized values, or another domain?
- Which CFA layouts and sample types does each algorithm support?
- How are borders, output dimensions, and channel ordering defined?
- What numeric range and rounding behavior does each algorithm produce?
- Which algorithm-specific options must remain outside the shared interface?
- What is the minimum common contract that permits independent
  implementations?
- Which initial algorithms are justified for `fornax`, and why?
- What does the existing `fornax-dalim` linear implementation already provide?

_No findings recorded yet._

## 2. Findings

Not started. Record only source-verified behavior here.

## 3. Evidence

Not started. Cite papers, reference implementations, source paths, symbols,
line ranges, and tests.

## 4. Implications for fornax

To be derived from the findings. Cover the shared input/output contract,
supported CFA and sample types, border policy, output representation, and
independent algorithm implementation.

## 5. Open questions and limitations

Record unresolved questions, unverified assumptions, and missing evidence.

## 6. References

- Papers with DOI or URL: not yet recorded.
- Reference implementations and commits: not yet recorded.
- Existing `fornax-dalim` files and tests: not yet recorded.
- License constraints: not yet recorded.
