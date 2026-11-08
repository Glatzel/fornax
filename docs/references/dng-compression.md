# DNG compression references

## Sources

- Adobe, *Digital Negative (DNG) Specification 1.7.1.0*, September 2023:
  <https://download.adobe.com/pub/adobe/dng/DNG_Spec_1_7_1_0.pdf>.
- Adobe, *TIFF 6.0 Specification*, 1992:
  <https://download.osgeo.org/libtiff/doc/TIFF6.pdf>.
- LibTIFF 4.0.10 release notes:
  <http://libtiff.maptools.org/v4.0.10.html>.
- RFC 8878, *Zstandard Compression and the `application/zstd` Media Type*:
  <https://www.rfc-editor.org/rfc/rfc8878>.

## Decisions

Fornax recognizes DNG compression 1 (uncompressed), 7 (lossless JPEG-92),
8 (Deflate), 34892 (lossy JPEG), and 52546 (JPEG XL), plus experimental
compression 50000 when the `zstd` feature is enabled. Unknown values remain
representable in `CompressionMethod::Unknown` and produce typed decode errors.

Compression 50002 is the legacy TIFF JPEG XL identifier. It is not yet
represented by `CompressionMethod`; supporting it requires mapping the same
JPEG XL payload decoder without treating `52546` as the only JPEG XL
identifier. Compression 52546 is the identifier defined by DNG 1.7.

TIFF compression 50000 is the de-facto libtiff/GDAL assignment for Zstandard,
not an Adobe-registered DNG compression identifier. Current Fornax writing
support remains feature-gated and is documented as experimental.

Codec availability is not an optional capability. Zstandard and every future
codec must be mandatory build dependencies without per-codec Cargo features or
runtime fallback errors. The current `zstd` feature, optional
`structured-zstd` dependency, `cfg` fallbacks, and feature-specific CI/build
flags are a pending migration, not the target architecture.

The production DNG path must not add `zstd-sys` or another C-backed codec.
LibRaw, darktable, and RawTherapee remain development-only comparison
references.

Fornax uses `structured-zstd`, a pure-Rust encoder/decoder. Each DNG tile is an
independent Zstandard frame so tile decoding remains parallel. The default
writer level is 3; LosslessJPEG remains the default interoperable compression.
