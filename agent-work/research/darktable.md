# darktable and RAW Preprocessing

> **Task:** `RES-04`
> **Owner:** primary agent
> **Status:** `in_progress`
> **Last updated:** 2026-10-23
> **Upstream versions:** record the darktable version or commit inspected
>
> **Acceptance criteria (authoritative version in the task tracker):**
>
> - [ ] The pre-demosaic pipeline order is documented with source paths and
>   symbols.
> - [ ] Black-level subtraction, clipping, normalization, and masked-pixel
>   handling are described numerically.
> - [ ] The boundary between sensor-domain preprocessing and later processing is
>   explicit.
> - [ ] Out-of-scope later operations are named.
> - [ ] Contradictions with LibRaw or DNG semantics are recorded.
> - [ ] All six required report sections are complete.

## 1. Scope and questions

Investigate darktable's actual RAW preprocessing pipeline, black-level
correction, CFA handling, and the boundary between sensor-domain preprocessing
and later image processing. Questions to answer:

- Which modules run before demosaicing, and in what order?
- How are black and white levels represented, including per-channel and
  spatially varying patterns?
- How are masked pixels and margins handled?
- How is the CFA phase tracked through rawprepare and demosaic?
- What exactly is subtracted, clipped, normalized, or rounded, and in which
  sample domain?
- Where is the boundary between RAW-domain preprocessing and operations that
  `fornax` must not apply?
- Does any reference to "darkroom" mean a different project or module?

_No findings recorded yet._

## 2. Findings

Not started. Record only source-verified behavior here.

## 3. Evidence

Not started. Cite darktable repository URL, version or commit, module source
paths, symbols, line ranges, and tests.

## 4. Implications for fornax

To be derived from the findings. Cover the pre-demosaic operation sequence,
black-level selection, normalization, and the explicit out-of-scope boundary.

## 5. Open questions and limitations

Record unresolved questions, unverified assumptions, and missing evidence.

## 6. References

- Upstream repository and commit: not yet recorded.
- Module source files and symbols: not yet recorded.
- Tests and documentation: not yet recorded.
- License constraints: not yet recorded.
