# DCB reference notes

This note records the authoritative material used to implement Fornax's
conventional 2x2 RGB Bayer DCB path and the license obligations attached to
that work.

## Sources

- LibRaw 0.22.3, `dcb` and its helper stages:
  [`src/demosaic/dcb_demosaic.cpp`](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/dcb_demosaic.cpp#L849).
- LibRaw 0.22.3, CFA border interpolation and `pre_interpolate`:
  [`src/demosaic/misc_demosaic.cpp`](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L21).
- LibRaw 0.22.3, comparison adapter entry point:
  [`crates/compare-demosaic/cpp/algos/libraw_dcb.cpp`](../../crates/compare-demosaic/cpp/algos/libraw_dcb.cpp).

The reference call is `pre_interpolate()` followed by `dcb(0, 1)`. The
iteration argument is zero, so the optional three-call `dcb_nyquist` sequence
is retained as a tested helper but is not executed by the production path.

## Algorithm shape

DCB first fills the six-pixel border with CFA-aware averages. It then builds
full-frame horizontal and vertical candidates:

1. Estimate green on the opposite axis and reconstruct red and blue relative
   to those green candidates.
2. Compare local color ranges and select the horizontal or vertical green
   candidate at each non-green site.
3. Save the current red and blue planes, reconstruct the missing colors, and
   smooth them using green contrast.
4. Apply one contrast-aware correction and three direction-map/correction
   passes.
5. Restore the saved red and blue planes, reconstruct color again, refine
   green with directional ratios and neighbor limits, then perform the final
   chroma-map interpolation and red/blue neighbor limit.

Fornax stores the working image as `[u16; 4]`: red at channel 0, unified green
at channel 1, blue at channel 2, and the direction map at channel 3. The
horizontal and vertical candidates use `[f32; 3]` full-frame planes. The
result is packed back to 16-bit RGB at the `DenseLazy` boundary.

## Numeric behavior

- Input and output are 16-bit codes in the comparison path.
- `CLIP(int(expression))` truncates toward zero and clamps to `0..=65535`.
- Direct `ushort(expression)` conversions truncate rather than round.
- The direction map stores zero for horizontal and one for vertical.
- Final full-color reconstruction is allowed to change sampled red or blue
  values through its eight-neighbor limits; Fornax does not restore CFA samples
  after that pass.

On the pinned `colorchart-eos-7d.cr2` fixture, all four Bayer phases match
LibRaw 0.22.3 exactly after the comparison harness converts between sample
types.

## License

LibRaw's `dcb_demosaic.cpp` retains the following BSD-3-Clause notice from the
DCB/FBDD author:

```text
Copyright (C) 2010, Jacek Gozdz (cuniek@kft.umcs.lublin.pl)

This code is licensed under a (3-clause) BSD license as follows:

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are met:

* Redistributions of source code must retain the above copyright notice, this
  list of conditions and the following disclaimer.
* Redistributions in binary form must reproduce the above copyright notice,
  this list of conditions and the following disclaimer in the documentation
  and/or other materials provided with the distribution.
* Neither the name of the author nor the names of its contributors may be used
  to endorse or promote products derived from this software without specific
  prior written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT OWNER OR CONTRIBUTORS BE LIABLE FOR
ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES
(INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES;
LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON
ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
(INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
```
