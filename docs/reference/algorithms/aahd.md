# AAHD reference notes

This note records the authoritative source and integer conventions used by
Fornax's conventional 2x2 RGB Bayer AAHD path.

## Source

- LibRaw 0.22.3, commit
  `cf02d66eb5c19d4e46df7aee0582e47a4918509a`,
  [`src/demosaic/aahd_demosaic.cpp`](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/aahd_demosaic.cpp#L771).
- LibRaw's comparison shim calls `pre_interpolate()` before
  `aahd_interpolate()`. For the fixture's three-color, full-resolution path,
  this unifies the two green CFA codes.

## Algorithm invariants

AAHD works on two full-frame directional RGB candidates with a four-pixel
zero-padded working margin:

1. Detect isolated hot or dead CFA samples and replace them only in the
   interpolation candidates.
2. Build horizontal and vertical green candidates with LibRaw's clipped
   second-order correction.
3. Fill the remaining red and blue values first along each candidate's primary
   direction, then choose low-gradient diagonal support within that candidate.
4. Convert both candidates through a 16-bit sRGB-to-linear lookup table, then
   calculate Rec. 2020 YUV values.
5. Count local directional homogeneity, break equal scores with a directional
   interpolation error, and refine the flags in two row-parity passes.
6. Select one complete RGB candidate for each pixel and restore the original
   measured sample at sites marked hot.

The implementation preserves LibRaw's integer arithmetic, channel extrema,
direction flags, and truncating conversion to `ushort`. The gamma table uses
the reference thresholds and coefficients: `0.0181`, `4.5`, `1.0993`, and
`0.45`.

## Fornax boundary

The comparison bridge supplies an identity camera-to-RGB matrix and no white
balance. Fornax targets that stage boundary and accepts only the four
conventional 2x2 RGB Bayer patterns. LibRaw's camera metadata, progress
callbacks, output-color processing, and non-Bayer fallbacks are outside this
implementation.

The comparison's native GBRG result is protected by both the 128x128
regression test and the canonical full-resolution run. The error map should
remain sparse texture noise; full-frame structure or a border strip indicates
a pipeline-stage regression.

## Licensing

LibRaw's `aahd_demosaic.cpp` is dual-licensed LGPL-2.1 or CDDL-1.0. This source
is used as the behavioral specification. A direct translation can remain a
derivative work, so project licensing review is required before distributing
the implementation.
