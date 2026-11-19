# Demosaic algorithm references

This index collects current demosaic implementation notes and retained source
provenance for conventional 2x2 RGB Bayer processing.

## Implemented algorithms

- [AMaZE reference notes](amaze.md)
- [LMMSE reference notes](lmmse.md)
- [VNG and VNG4 reference notes](vng.md)

Linear, PPG, RCD, AMaZE, VNG, VNG4, and the darktable-compatible LMMSE path are
implemented by `fornax-raw-pipeline`. Source research remains separate from the
runtime contracts in `fornax-raw-core`.

## Removed implementation provenance

- [AAHD reference notes](aahd.md)
- [DCB reference notes](dcb.md)
- [DHT reference notes](dht.md)

These notes preserve source and numerical evidence for formulations removed
from the production API. They are not descriptions of available selectors.

## External reference trees

- [RawTherapee reference notes](rawtherapee.md)

External projects provide behavioral and source evidence only. They are not
production dependencies.

## Tier 1 eligibility audit

The current research audit covers Malvar-He-Cutler, patterned pixel grouping,
ratio-corrected demosaicing, and directional LMMSE. The individual reports and
cross-algorithm matrix are added alongside this index.
