# DNG Format

This page summarizes the implemented DNG decoder scope and future acceptance
boundary.

**Tier:** 0. **Specification:** Adobe DNG 1.7.1.0. Canonical detail:
[DNG decoder reference notes](../../reference/decoders/reference/dng-decoder.md).

## Supported

- Classic TIFF in either byte order, BigTIFF, root IFD chains, and `SubIFDs`.
- Full-resolution CFA/`LinearRaw` selection with checked strip/tile geometry.
- Uncompressed integer/float, lossless JPEG-92 predictors 1-7, Deflate,
  single-component lossy JPEG, JPEG XL, and experimental Zstandard chunks.
- 2x2 interleave, linearization, `OpcodeList1`, parallel chunk decode, typed
  metadata preservation, and conventional 2x2 Bayer handoff.
- Uncompressed DNG plus tiled lossless JPEG-92 or Deflate regeneration.

Export is regenerated, not byte-identical. Compressions `50002` and `52546`
are the TIFF and DNG JPEG XL identifiers; Zstandard requires its feature.

## Remaining limits

Multi-channel `LinearRaw`, non-2x2 interleave, lossless-JPEG restart/selector
and multi-component tile variants, downstream opcode stages, and automatic
BigTIFF output are outside this slice. Previews, masks, depth, nested Exif/GPS,
and other auxiliary directories are not regenerated.

## Acceptance

Synthetic tests must cover newly supported storage and compression, CFA
round-trip, malformed metadata, and typed unsupported handoff. Reviewed corpus
rows must preserve exact samples where a reference comparison exists. DNG
production dependencies remain free of C/C++ and `-sys` crates.
