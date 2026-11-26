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
| [`RCD`](rcd.md) | Ready; done | Supported linear-light RCD 2.3 formulation |
| [`LMMSE`](lmmse.md) | Ready; done | Supported linear-light Zhang-Wu LMMSE formulation |
| [`AMaZE`](amaze.md) | Ready; done | Supported linear-light AMaZE formulation |
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
| [Original Pixel Grouping](ppg.md#sources) | Eligible | Yes | No | Yes | Yes; monotonic ratio branch has no pole | Yes | Not separately implemented |
| [Modern LibRaw/darktable PPG](ppg.md#algorithm) | Eligible | Yes | No | Yes | Yes | Yes | Ready as `Demosaic::Ppg` |
| [RCD 2.3](rcd.md) | Ready | Yes | No | Yes (tile-normalized) | Yes (zero-crossing fallback) | Yes | Ready as `Demosaic::Rcd` |
| [Zhang-Wu LMMSE](lmmse.md) | Ready | Yes | No | Yes (tile-normalized) | Yes | Yes | Ready as `Demosaic::Lmmse` |
| [AMaZE](amaze.md) | Ready | Yes | No | Yes (tile-normalized) | Yes | Yes | Ready as `Demosaic::Amaze` |

## Algorithms to remove

None of the three Tier 1 mathematical families is fundamentally ineligible.
MHC remains useful reference research, but its patent and source-license risk
place it below the implementation tiers.

## Mandatory implementation changes

### MHC

- Keep the equations and source provenance as research evidence only.
- Do not implement, port, or copy MHC code into Fornax.

### Modern PPG

- Readiness complete: keep the modern equations and linear-light arithmetic.
- Horizontal-first and diagonal-tie behavior are compatibility contracts.

### RCD

- Readiness complete: linear-light RCD 2.3 with symmetric `epssq` energy
  floors, parity-accurate diagonal HPF indexing, and sign-safe ratio / additive
  zero-crossing fallback.

### Zhang-Wu LMMSE

- Readiness complete: linear-light Zhang-Wu LMMSE with tile-normalized variance
  floor, parity-preserving reflected halos, and configurable `LmmseOptions`.

### AMaZE

- Readiness complete: linear-light AMaZE with tile normalization, out-of-place
  Jacobi stencil passes, corrected 0.25 variance means, symmetric chroma
  taps, sign-safe ratio fallback, unclamped neighbor overshoot bounds, and
  configurable `AmazeOptions`.

## Unresolved research questions

None for Linear, PPG, RCD, LMMSE, or AMaZE. Remaining `Not ready` algorithms
(`VNG`, `VNG4`) are tracked separately.

## Individual reports

- [Linear demosaicing readiness](linear.md)
- [Malvar-He-Cutler eligibility](mhc.md)
- [Patterned pixel grouping readiness](ppg.md)
- [Ratio-corrected demosaicing readiness](rcd.md)
- [Directional LMMSE readiness](lmmse.md)
- [AMaZE readiness](amaze.md)

## Other implementation notes

- [VNG and VNG4 reference notes](vng.md)
- [RawTherapee reference notes](rawtherapee.md)

## Removed implementation provenance

- [AAHD reference notes](aahd.md)
- [DCB reference notes](dcb.md)
- [DHT reference notes](dht.md)

These notes preserve source and numerical evidence for formulations removed
from the production API. They are not descriptions of available selectors.
