# Demosaic algorithm references

This index collects demosaic source research, eligibility decisions, and
retained provenance for conventional 2x2 RGB Bayer processing.

## Production readiness

Readiness is narrower than mathematical eligibility or implementation
availability.

| Algorithm or selector | Status | Decision |
| --- | --- | --- |
| [`Linear`](linear.md) | Ready; done | Supported bilinear baseline |
| Modern LibRaw/darktable [`PPG`](ppg.md) | Ready; done | Supported PPG formulation |
| `AMaZE` | Not ready | Implemented and selectable; follow-up work remains |
| `RCD` | Not ready | Conversion details and regression coverage remain |
| `LMMSE` | Not ready | Gamma-workspace compatibility variant remains |
| `VNG` | Not ready | Implemented and selectable; follow-up work remains |
| `VNG4` | Not ready | Implemented and selectable; follow-up work remains |
| Malvar-He-Cutler | Tier -1 | Reference only; never implement |

## Tier -1 reference-only

| Algorithm | Mathematical status | Disposition |
| --- | --- | --- |
| [Malvar-He-Cutler](mhc.md) | Affine linear reconstruction | Reference only; never implement because of patent and source-license risk |

## Tier 1 eligibility report

The audit uses four verdicts:

- **Eligible:** naturally compatible with Fornax's linear-light contract.
- **Convertible:** compatible after a mathematically justified change.
- **Conditional:** requires a documented restriction or distinct named variant.
- **Ineligible:** fundamentally depends on prohibited processing.

Verdicts describe the mathematical family. Current Fornax behavior is reported
separately because implementation choices such as integer storage, bounded
LUTs, or gamma workspaces do not by themselves determine mathematical
eligibility.

| Algorithm or variant | Verdict | Linear-light reconstruction | Fixed input range | Positive scale equivariance | Negative finite inputs | Generic `f32`/`f64` | Current Fornax status |
| --- | --- | --- | --- | --- | --- | --- | --- |
| [Original Pixel Grouping](ppg.md#original-pixel-grouping-equations) | Eligible | Yes | No | Yes | Yes; monotonic ratio branch has no pole | Yes | Not separately implemented |
| [Modern LibRaw/darktable PPG](ppg.md#modern-librawdarktable-ppg-equations) | Eligible | Yes | No | Yes | Yes | Yes | Ready as `Demosaic::Ppg` |
| [RCD 2.3](rcd.md) | Convertible | Yes | No | Yes with scale-relative tolerances | Needs a defined denominator fallback | Yes | Implemented with most conversion work completed |
| [Zhang-Wu LMMSE](lmmse.md) | Convertible | Yes | No | Yes with a relative variance floor | Yes | Yes | Not the current default formulation |
| [darktable-compatible LMMSE](lmmse.md#current-fornax-behavior) | Conditional | Encoded workspace, decoded output | No, but globally scale-normalized | Yes through normalization | Yes through odd transfer extension | Yes | Current `Demosaic::Lmmse` |

## Algorithms to remove

None of the three Tier 1 mathematical families is fundamentally ineligible.
MHC remains useful reference research, but its patent and source-license risk
place it below the implementation tiers.

This is not a recommendation to retain every current implementation unchanged.
The following implementation claims must change:

- Canonical LMMSE must not be described as operating in a gamma domain.
- The current gamma-domain LMMSE must not be the unqualified canonical
  formulation.
- RCD must not rely on a fixed reference epsilon at signed ratio
  singularities.

## Algorithms that require named variants

- **`LMMSE-darktable` or equivalent:** the current gamma-workspace, PPG-seeded,
  median/refinement-compatible path may survive only as an explicitly named
  compatibility variant.
- **`PixelGrouping`:** historical only. Modern `PPG` is sufficient and a
  separate original-formulation selector is not planned.

## Mandatory implementation changes

### MHC

- Keep the equations and source provenance as research evidence only.
- Do not implement, port, or copy MHC code into Fornax.

### Modern PPG

- Readiness complete: keep the modern equations and linear-light arithmetic.
- Horizontal-first and diagonal-tie behavior are compatibility contracts.

### Original Pixel Grouping

- Not planned. The equations remain historical context for the ambiguous
  `PPG` name.

### RCD

- Replace fixed/reference-domain tolerances with scale-relative values.
- Define a finite fallback when `L_0 + L_2` crosses zero.
- Keep only discrimination-weight clamps in `[0,1]`; remove RGB range clamps.
- Test signed low-pass cancellation, HDR, scaling, and `f64` behavior.

### Canonical LMMSE

- Evaluate directional estimates, local variances, fusion, and
  color-difference reconstruction directly on linear samples.
- Replace fixed variance additions with a scale-relative floor.
- Define the zero-variance and all-zero fallback.
- Preserve signed values and HDR range without a nonlinear intermediate.

### darktable-compatible LMMSE

- Rename or otherwise isolate its gamma-workspace behavior.
- Keep the reversible transfer, PPG seed, bounded difference policies, median
  passes, and refinement passes outside the canonical LMMSE claim.

## Unresolved research questions

- What finite fallback should RCD use at a signed ratio denominator crossing?
- What scale-relative variance floor preserves Zhang-Wu behavior in `f32`?
- Should canonical linear LMMSE become the default while darktable compatibility
  remains a named selector?
- Which independent references replace the removed custom comparison harness?

## Individual reports

- [Linear demosaicing readiness](linear.md)
- [Malvar-He-Cutler eligibility](mhc.md)
- [Patterned pixel grouping eligibility](ppg.md)
- [Ratio-corrected demosaicing eligibility](rcd.md)
- [Directional LMMSE eligibility](lmmse.md)

## Other implementation notes

- [AMaZE reference notes](amaze.md)
- [VNG and VNG4 reference notes](vng.md)
- [RawTherapee reference notes](rawtherapee.md)

## Removed implementation provenance

- [AAHD reference notes](aahd.md)
- [DCB reference notes](dcb.md)
- [DHT reference notes](dht.md)

These notes preserve source and numerical evidence for formulations removed
from the production API. They are not descriptions of available selectors.
