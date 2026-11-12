# RawTherapee reference notes

This note records the pinned RawTherapee source used by `compare-demosaic` and
the boundary conditions required to reproduce its demosaic routines without
building the complete RawTherapee application.

## Sources

RawTherapee release 5.13 is pinned at commit
`1924179107995b01b73cdc7f4fe91e44fc8227a7`.

- AHD, `ahd_demosaic`:
  [rtengine/ahd_demosaic_RT.cc](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/ahd_demosaic_RT.cc#L44)
- AMaZE, `amaze_demosaic_RT`:
  [rtengine/amaze_demosaic_RT.cc](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/amaze_demosaic_RT.cc#L48)
- IGV and DCB:
  [rtengine/demosaic_algos.cc](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/demosaic_algos.cc#L218)
- EAHD, `eahd_demosaic`:
  [rtengine/eahd_demosaic.cc](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/eahd_demosaic.cc#L214)
- HPHD, `hphd_demosaic`:
  [rtengine/hphd_demosaic_RT.cc](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/hphd_demosaic_RT.cc#L290)
- LMMSE, `lmmse_interpolate_omp`:
  [rtengine/lmmse_demosaic.cc](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/lmmse_demosaic.cc#L42)
- RCD, `rcd_demosaic`:
  [rtengine/rcd_demosaic.cc](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/rcd_demosaic.cc#L53)
- VNG4, `vng4_demosaic`:
  [rtengine/vng4_demosaic_RT.cc](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/vng4_demosaic_RT.cc#L64)

`scripts/setup.sh` and `scripts/setup.ps1` materialize only these sources and
their direct support headers. The setup scripts and `build.rs` hold the same
release and commit constants.

## Comparison boundary

The RawTherapee adapters accept the comparison harness's conventional 2x2 RGB
Bayer mosaic and return 16-bit RGB. They normalize LibRaw's split-green filter
mask for routines that require one green code, while VNG4 retains the original
four-code `prefilters` mask because its interpolation uses separate green
planes.

The adapters supply the active-area codes directly, an identity camera matrix,
no white balance, and no RawTherapee decode or output-color processing. Sources
are compiled through a compatibility facade for array, color, LUT, and progress
interfaces. Scalar source branches are selected to avoid GTK, GLib, FFTW, and
SLEEF build dependencies.

Method parameters are fixed for comparison:

- AMaZE uses two-tile scheduling and `initialGain = 1`;
- DCB uses zero refinement iterations with enhancement enabled;
- LMMSE uses one iteration;
- RCD uses two-tile scheduling;
- AHD, VNG4, IGV, EAHD, and HPHD have no additional method parameters.

The RawTherapee 5.13 AMaZE scalar path can write one pixel past the output plane
for odd-width GRBG crops. The compatibility object allocates two guard
rows/columns for RGB output and the bridge copies only the requested dimensions.
This preserves the reference result while making non-native Bayer crops safe.

The compatibility `xdivf(value, exponent)` follows RawTherapee's exponent-shift
semantics. `xdivf(value, 2)` divides by four rather than by two.

## Known differences

RawTherapee normally runs demosaicing after `scaleColors`, which applies camera
gain and white-balance channel scaling. The comparison deliberately does not
apply white balance so all references receive the same active mosaic. Methods
whose decisions depend strongly on channel scaling, especially AMaZE, can
therefore differ substantially from Fornax even though sampled CFA values
remain exact.

The facade also omits RawTherapee progress reporting, metadata, camera profiles,
and surrounding preprocess/postprocess stages. Differences caused by those
stages are outside the demosaic comparison.

Dual demosaic is intentionally excluded. This includes Dual's bilinear and VNG4
blends; processing any one of its component methods does not enable Dual.

## Licensing

RawTherapee is licensed under GPL-3.0-or-later. Its pinned source is a reference
dependency of the non-published `compare-demosaic` development binary and is
not linked into Fornax's Rust library crates. The compatibility files derived
from RawTherapee interfaces retain their upstream GPL notices. Any direct reuse
of RawTherapee source in a distributed artifact requires GPL-compatible
licensing review.
