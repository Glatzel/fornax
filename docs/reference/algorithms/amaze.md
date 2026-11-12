# AMaZE reference notes

This reference note records pinned sources, numerical behavior, and implementation boundaries for AMaZE.

This note records the authoritative material and conventions used by Fornax's
conventional 2x2 RGB Bayer AMaZE path.

## Sources

- Emil Martinec, *AMaZE: Aliasing Minimization and Zipper Elimination*,
  implementation dated May 27, 2010 and later optimized by Ingo Weyrich:
  [RawTherapee `amaze_demosaic_RT`](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/amaze_demosaic_RT.cc#L48).
- darktable 5.6.2, `amaze_demosaic`:
  [src/iop/demosaicing/amaze.cc](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/amaze.cc#L129).

Both are pinned comparison references. darktable is the primary acceptance
oracle; RawTherapee exercises the independently maintained implementation of
the same published algorithm.

## Algorithm shape

AMaZE reconstructs green with adaptive color ratios and Hamilton-Adams
candidates in the four cardinal directions. Local variance and directional
fluctuation estimates choose the vertical/horizontal mix. A Nyquist-texture
test detects alternating color-difference energy, after which area
interpolation and green-curvature refinement replace the normal directional
choice. Diagonal red/blue ratios are then bounded against local CFA extrema,
followed by a second green refinement from interpolated red-plus-blue and a
quincunx chroma interpolation.

The tile geometry is part of the comparison behavior:

- tile size: 160 pixels;
- halo: 16 pixels;
- first tile origin: `(-16, -16)`;
- tile stride: 128 pixels;
- right and bottom image edges: reference reflection rules;
- final output: normalized `f32`, clamped once at the lazy boundary.

Fornax reuses one tile workspace, including scratch state that is not cleared
between tiles. Resetting all planes per tile changes partial-tile results.
Pixels in AMaZE's eight-bit Nyquist masks and half-resolution difference planes
are addressed with the upstream flattened-index convention rather than by
treating those planes as full-resolution images.

## Fornax differences and conventions

The comparison supplies an identity camera matrix, no white balance, and
normalized active-area Bayer data. Those pipeline decisions are outside the
typed Fornax demosaic stage. Fornax applies the same normalization, ratio
thresholds, tile reflections, clipping points, and nearest-code quantization
as the comparison adapters.

The RawTherapee compatibility facade implements `xdivf(value, exponent)` with
the upstream exponent-shift meaning. In particular, `xdivf(value, 2)` divides
by four. Treating the second argument as a divisor creates a recursive bias in
the half-resolution directional weights and prevents the reference from
matching either Fornax or darktable.

## Licensing

RawTherapee is GPL-3.0-or-later and darktable is GPLv3. Their source is used as
behavioral and specification evidence. The Fornax implementation is written in
Rust around the published algorithm and documented reference behavior. Any
direct reuse of reference source text still requires project licensing review.
