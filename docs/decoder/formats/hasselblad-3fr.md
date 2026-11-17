# Hasselblad 3FR Format

This page summarizes the accepted 3FR container/codec boundary.

**Tier:** 2. **Status:** accepted conventional-Bayer corpus. Canonical detail:
[Hasselblad 3FR reference notes](../../reference/decoders/reference/hasselblad-3fr.md).

## Supported

- Classic little-endian TIFF with a full-resolution CFA SubIFD.
- Compression 1 as exact little-endian 16-bit words.
- Compression 7 as single-component predictor-8 lossless JPEG with pair resets
  and 3FR bit order.
- Explicit RGGB profiles for accepted tagless models, including X1D-family
  border removal; source CFA tags take precedence when present.

## Unsupported

FFF, merged/multi-shot variants, tiled storage, multi-sample or non-16-bit
payloads, restart/point transforms, unknown profiles/compression, and non-2x2
pipeline input are typed unsupported.

## Acceptance

All 11 corpus cameras must be manifest-listed, uncompressed and predictor-8
samples must match the low-level reference, DNG round-trip must preserve
samples, and tracker rows must pass.
