# Sony ARW Format

This page summarizes accepted Sony ARW/NRW codec families and rejection
boundaries.

**Tier:** 1. **Status:** uncompressed, ARW1, ARW2, ARW6, and lossless-JPEG CFA.
Canonical detail:
[Sony ARW/NRW reference notes](../../reference/decoders/reference/sony-arw.md).

## Supported

- Uncompressed 12-bit packed and 14/16-bit word samples.
- ARW1 and ARW2 nonlinear/bit-packed streams with camera curves.
- ARW6 LLVC3 wavelet streams.
- Single-component and Sony four-component tiled lossless JPEG, including the
  four-component-to-two-Bayer-row reshape.
- Checked private-data decryption and conventional 2x2 Bayer handoff.

## Unsupported

ARQ pixel-shift/multi-sample files, reduced-resolution or YCbCr `LinearRaw`
medium/small modes, unknown compression, non-Bayer layouts, and ambiguous
primary mosaic selection return typed unsupported errors. Preview and
auxiliary images are never raw sources.

## Acceptance

Accepted families need fixture-backed tests and passing tracker rows. Samples
must match Rawler or LibRaw after curve, levels, crop, CFA, and endianness
alignment. Unsupported variants must return typed errors. Synthetic tests cover
private offsets, packed rows, tile geometry, LJPEG row expansion, and Bayer
handoff.
