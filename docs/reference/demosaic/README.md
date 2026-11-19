# Demosaic algorithm references

This index collects demosaic source research, eligibility decisions, and
retained provenance for conventional 2x2 RGB Bayer processing.

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
| [Malvar-He-Cutler](mhc.md) | Eligible | Yes | No | Yes | Yes | Yes | No kernel; Tier 1 candidate |
| [Original Pixel Grouping](ppg.md#original-pixel-grouping-equations) | Eligible | Yes | No | Yes | Yes; monotonic ratio branch has no pole | Yes | Not separately implemented |
| [Modern LibRaw/darktable PPG](ppg.md#modern-librawdarktable-ppg-equations) | Eligible | Yes | No | Yes | Yes | Yes | Implemented as `Demosaic::Ppg` |
| [RCD 2.3](rcd.md) | Convertible | Yes | No | Yes with scale-relative tolerances | Needs a defined denominator fallback | Yes | Implemented with most conversion work completed |
| [Zhang-Wu LMMSE](lmmse.md) | Convertible | Yes | No | Yes with a relative variance floor | Yes | Yes | Not the current default formulation |
| [darktable-compatible LMMSE](lmmse.md#current-fornax-behavior) | Conditional | Encoded workspace, decoded output | No, but globally scale-normalized | Yes through normalization | Yes through odd transfer extension | Yes | Current `Demosaic::Lmmse` |

## Algorithms to remove

None of the four Tier 1 mathematical families is fundamentally ineligible.
The audit does not justify deleting MHC, either Pixel Grouping formulation,
RCD, or canonical LMMSE from the research set.

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
- **`PixelGrouping`:** if the original 2003 four-gradient and `hue_transit`
  equations are added, they require a name distinct from modern `PPG`.

## Mandatory implementation changes

### MHC

- Implement the eight affine 5x5 cases directly in generic floating-point
  arithmetic.
- Define finite-image boundary normalization and tiny-image behavior.
- Do not restore display clamps or normalized-code conversion.
- Complete patent and source-license review before accepting the
  implementation.

### Modern PPG

- Keep the existing modern equations and linear-light arithmetic.
- Document horizontal-first and diagonal-tie behavior as compatibility
  contracts.
- Add no conversion solely for eligibility.

### Original Pixel Grouping

- Implement its four-gradient green equations and guarded `hue_transit`
  formulation under a separate name.
- Define a finite-image border rule and direction tie rule.
- Add the same signed, scale, HDR, precision, and boundary tests as other
  retained kernels.

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

- What boundary oracle should define MHC at the outer two pixels?
- What patent posture does Fornax want before accepting MHC?
- Should original Pixel Grouping be implemented separately from modern `PPG`?
- What finite fallback should RCD use at a signed ratio denominator crossing?
- What scale-relative variance floor preserves Zhang-Wu behavior in `f32`?
- Should canonical linear LMMSE become the default while darktable compatibility
  remains a named selector?
- Which independent references replace the removed custom comparison harness?

## Individual reports

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
