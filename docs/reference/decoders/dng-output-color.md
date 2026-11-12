# DNG output color management

## Sources

- Adobe, *Digital Negative (DNG) Specification 1.7.1.0*, chapter 6,
  “Mapping Camera Color Space to CIE XYZ Space”:
  <https://download.adobe.com/pub/adobe/dng/DNG_Spec_1_7_1_0.pdf>.
- IEC 61966-2-1, sRGB color space and transfer function.

## Fornax output transform

Fornax does not interpolate Matrix1/Matrix2 by illuminant. It selects the
complete first profile when available, then falls back to the complete second
profile, and uses the calibration paired with the selected color matrix.

Fornax preserves `ColorMatrix1`, `ColorMatrix2`, `ForwardMatrix1`,
`ForwardMatrix2`, `CameraCalibration1`, and `CameraCalibration2` in
`ProcessMetadata` rather than collapsing them into a single predefined
transform.

1. Prefer `ForwardMatrix1`, which maps white-balanced camera coordinates to
   D50 XYZ.
2. Fall back to `ForwardMatrix2`.
3. If no forward matrix exists, multiply `ColorMatrix1` or `ColorMatrix2` by
   its matching camera calibration and invert the result. An absent
   calibration is the DNG-specified identity matrix.
4. Compose the camera-to-XYZ matrix with the selected D50 XYZ-to-output matrix.
5. Apply the resulting matrix after demosaic and green-normalized white
   balance when `PostProcessor::output_color` is not `Raw`.

The selectable scene-linear outputs are sRGB, Adobe RGB (1998), Adobe Wide
Gamut RGB, ProPhoto RGB, CIE XYZ, ACES2065-1, ACEScg, DCI-P3, and Rec. 2020.
sRGB, Adobe RGB, ACES, and Rec. 2020 matrices use Bradford-adapted D50 input;
Wide Gamut RGB and ProPhoto RGB use their native D50 white points; XYZ is
passed through.

Output transfer functions, analog balance, profile tone curves, and a complete
DNG render pipeline remain outside this implementation. If no usable camera
color matrix is present, the output remains camera RGB even when another
output color is requested.

## Matrix sources

- IEC 61966-2-1 for the sRGB primaries and D50-adapted XYZ transform.
- Adobe, *Adobe RGB (1998) Color Image Encoding*, for Adobe RGB primaries.
- Adobe, *Digital Negative (DNG) Specification 1.7.1.0*, chapter 6, for color
  and forward matrix orientation, calibration, and D50 XYZ.
- SMPTE ST 431-2 for DCI-P3 primaries and cinema white point.
- ITU-R BT.2020 for Rec. 2020 primaries.
- Academy S-2014-004 for ACES2065-1 and ACEScg primaries and white points.

The constants use Bradford chromatic adaptation from D50 XYZ to D65 or D60
where the target's native white point requires it. Fornax stores values to
ten decimal places, which keeps the matrices synchronous while preserving
float32 precision.
