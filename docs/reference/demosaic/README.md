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
| [`DHT`](dht.md) | Ready; done | Supported clean-room linear-light DHT formulation |
| [`VNG`](vng.md) | Ready; done | Supported linear-light 3-color VNG formulation |
| [`VNG4`](vng.md) | Ready; done | Supported linear-light 4-color split-green VNG4 formulation |
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
| [Clean-room DHT](dht.md) | Ready | Yes | No | Yes (tile-normalized) | Yes (zero-crossing fallback) | Yes | Ready as `Demosaic::Dht` |
| [VNG (3-color)](vng.md) | Ready | Yes | No | Yes | Yes | Yes | Ready as `Demosaic::Vng` |
| [VNG4 (4-color split-green)](vng.md) | Ready | Yes | No | Yes | Yes | Yes | Ready as `Demosaic::Vng4` |

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

### Clean-room DHT

- Readiness complete: clean-room linear-light DHT with tile normalization,
  symmetric 7-tap HV directional costs, 3-pass Jacobi majority-vote refinement,
  sign-safe ratio / additive zero-crossing green reconstruction, symmetric
  diagonal costs with 1-pass refinement, inverse-gradient additive R/B
  reconstruction, and configurable `DhtOptions`.

### VNG and VNG4

- Readiness complete: linear-light 3-color `Demosaic::Vng` and 4-color
  split-green `Demosaic::Vng4` sharing one streaming `VngLazy` kernel with
  parity-preserving reflected borders and canonical `minimum + 0.5 * maximum`
  thresholding.

## Unresolved research questions

None. All implemented production demosaic selectors (`Linear`, `Ppg`, `Rcd`,
`Lmmse`, `Amaze`, `Dht`, `Vng`, `Vng4`) are ready.

## Individual reports

- [Linear demosaicing readiness](linear.md)
- [Malvar-He-Cutler eligibility](mhc.md)
- [Patterned pixel grouping readiness](ppg.md)
- [Ratio-corrected demosaicing readiness](rcd.md)
- [Directional LMMSE readiness](lmmse.md)
- [AMaZE readiness](amaze.md)
- [DHT readiness](dht.md)
- [VNG and VNG4 readiness](vng.md)

## Other implementation notes

- [RawTherapee reference notes](rawtherapee.md)

## Removed implementation provenance

- [AAHD reference notes](aahd.md)
- [DCB reference notes](dcb.md)

These notes preserve source and numerical evidence for formulations removed
from the production API. They are not descriptions of available selectors.
