# DHT reference notes

This note records the authoritative source and numeric conventions used by
Fornax's conventional 2x2 RGB Bayer DHT path.

## Source

- LibRaw 0.22.3, commit
  `cf02d66eb5c19d4e46df7aee0582e47a4918509a`,
  [`src/demosaic/dht_demosaic.cpp`](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/dht_demosaic.cpp).
- LibRaw 0.22.3, three-color CFA preprocessing:
  [`src/demosaic/misc_demosaic.cpp`](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L21).
- Fornax comparison adapter:
  [`crates/compare-demosaic/cpp/algos/libraw_dht.cpp`](../../crates/compare-demosaic/cpp/algos/libraw_dht.cpp).

The comparison calls `pre_interpolate()` followed by `dht_interpolate()`. For
the fixture's three-color, full-resolution Bayer path, preprocessing unifies
the two green CFA phases. DHT's fallback to AHD for unsupported filter arrays
is outside the Fornax scope.

## Algorithm shape

DHT works in a float RGB plane with four padded rows and columns on the top and
left. Padding is initialized to `0.5`, while measured CFA values replace their
channel after nonzero bounds are collected.

The fixed reference sequence is:

1. Detect isolated hot or dead CFA samples using eight neighbors and a
   one-stop `calc_dist` threshold. Replace candidate values along the
   lower-distance axis, preserving a hot-sample flag for final restoration.
2. Discover horizontal and vertical directions from ratios of green and
   known non-green samples. The direction confidence is raised to the eighth
   power, and ratios above `256` become strong-direction flags.
3. Refine HV directions in two row-parity passes and then in a column pass.
4. Interpolate missing green values along the selected axis, apply the
   `scale_under`/`scale_over` correction, and clamp to channel bounds.
5. Discover diagonal directions, refine unsupported diagonals, and
   interpolate the missing red or blue sample at non-green sites.
6. Reconstruct red and blue at green sites along the selected HV direction.
7. Restore measured samples at hot sites and convert each float channel to
   `ushort` by truncation.

The implementation uses the reference flag values, constants, and traversal
order. Its public boundary is still normalized input and normalized RGB output;
the internal integer comparison behavior is preserved by converting through
16-bit codes at the `DenseLazy` boundary.

## Numeric behavior

- `calc_dist` returns a ratio and is invariant to one-stop scaling.
- The hot threshold is `64`, the strong HV threshold is `256`, and the strong
  diagonal threshold is `1.4`.
- Green interpolation applies squared ratio weights; diagonal red/blue
  interpolation applies cubed ratio weights; HV red/blue interpolation applies
  squared ratio weights.
- Direct float-to-`u16` output truncates toward zero and saturates at the
  unsigned 16-bit limits.
- Measured Bayer samples are not changed by the interpolation stages, except
  that hot samples are intentionally restored after candidate interpolation.

## Fornax boundary

The comparison bridge supplies an identity camera-to-RGB matrix and no white
balance. Fornax supports only RGGB, BGGR, GRBG, and GBRG. Camera metadata,
LibRaw callbacks, shrink, half-size, four-color RGB, output color processing,
and the AHD fallback are outside this implementation.

Rare interpolated-site differences remain when independent float evaluation
places a local direction on the other side of a comparison boundary. The
native GBRG row and all cropped consistency rows remain well inside the robust
limits, with sampled Bayer sites exact.

## Licensing

LibRaw's `dht_demosaic.cpp` retains the following notice:

```text
Copyright 2013 Anton Petrusevich

This code is licensed under one of two licenses as you choose:

1. GNU LESSER GENERAL PUBLIC LICENSE version 2.1
2. COMMON DEVELOPMENT AND DISTRIBUTION LICENSE (CDDL) Version 1.0
```

The source is used as the behavioral specification for a direct Rust
translation, which can remain a derivative work. Redistribution must therefore
retain the copyright and comply with one of the source's dual-license choices;
project licensing review is required before distributing the implementation.
