# DNG compression references

## Sources

- Adobe, *Digital Negative (DNG) Specification 1.7.1.0*, September 2023:
  <https://download.adobe.com/pub/adobe/dng/DNG_Spec_1_7_1_0.pdf>.
- Adobe, *TIFF 6.0 Specification*, 1992:
  <https://download.osgeo.org/libtiff/doc/TIFF6.pdf>.
- Adobe, *TIFF Technical Note #2*, 17 March 1995:
  <https://libtiff.gitlab.io/libtiff/specification/technote2.html>.
- LibTIFF 4.0.10 release notes:
  <http://libtiff.maptools.org/v4.0.10.html>.
- GDAL maintainer discussion of TIFF JPEG XL identifier `50002` and DNG
  identifier `52546`:
  <https://www.mail-archive.com/gdal-dev@lists.osgeo.org/msg42055.html>.
- RFC 8878, *Zstandard Compression and the `application/zstd` Media Type*:
  <https://www.rfc-editor.org/rfc/rfc8878>.
- Sony ARW/NRW behavior and pinned vendor-reference notes:
  [sony-arw.md](sony-arw.md).

## Decisions

Fornax recognizes DNG compression 1 (uncompressed), 7 (lossless JPEG-92),
8 (Deflate), 34892 (lossy JPEG), and JPEG XL identifiers 50002 and 52546, plus
experimental Zstandard compression 50000. Unknown DNG and compatibility IDs
remain representable in `DngCompressionMethod::Unknown`; unknown generic TIFF
IDs remain representable in `CompressionMethod::Unknown`. Both domains produce
typed decode errors.

Compression 50002 is the legacy TIFF JPEG XL identifier. Compression 52546 is
the identifier defined by DNG 1.7. Both map to the same JPEG XL payload decoder.

TIFF compression 50000 is the de-facto libtiff/GDAL assignment for Zstandard,
not an Adobe-registered DNG compression identifier. Fornax support remains
documented as experimental.

Codec availability is not an optional capability. Zstandard and every future
codec are mandatory build dependencies without per-codec Cargo features or
runtime fallback errors.

The production DNG path must not add `zstd-sys` or another C-backed codec.
LibRaw, darktable, and RawTherapee remain development-only comparison
references.

Fornax uses `structured-zstd`, a pure-Rust encoder/decoder. Each DNG tile is an
independent Zstandard frame so tile decoding remains parallel. The default
writer level is 3; LosslessJPEG remains the default interoperable compression.

## Compression ownership

This table classifies every compression ID currently mapped by Fornax. The
numeric meaning and ownership were checked against the specifications and
vendor references listed below rather than derived from enum names or existing
comments.

| ID | Normative meaning and source | Supported data/layout | Current behavior | Proposed ownership |
| ---: | --- | --- | --- | --- |
| `1` | TIFF 6.0 uncompressed, tightly packed; TIFF 6.0 section 8 and Compression field on p.30 | Mono integer 1-32-bit or float 16/24/32-bit in Fornax; strips or tiles | DNG maps to `DecodeCompression::Uncompressed`; DNG writer emits `1` | `fornax-tiff` default resolver to `Uncompressed` |
| `7` | TIFF Technical Note #2 complete ISO JPEG segment, sequential lossy or lossless; DNG 1.7.1 pp.20-21 requires lossless Huffman JPEG for CFA raw payloads | Fornax mono lossless JPEG-92, 2-16-bit, strips or tiles; ARW has a vendor-specific LJPEG tile layout | DNG maps to `LosslessJpeg`; ARW maps to Sony LJPEG; DNG writer emits `7` | `fornax-tiff` default resolver to `LosslessJpeg` for the supported mono raw subset; DNG keeps applicability policy; ARW keeps vendor semantics |
| `8` | Adobe Deflate extension; DNG 1.7.1 pp.20-21 defines Deflate (ZIP) and permitted DNG storage types; LibTIFF identifies `COMPRESSION_ADOBE_DEFLATE = 8` | Integer and float payloads with TIFF predictors; strips or tiles | DNG maps to `DecodeCompression::Deflate`; DNG writer emits `8` | `fornax-tiff` default resolver to `Deflate`; DNG retains version and sample-type policy |
| `32766` | Sony-private ARW6 compression, not TIFF-registered; pinned dnglab/Rawler and LibRaw behavior in `sony-arw.md` | Sony wavelet stream decoded to one `u16` mosaic | `fornax-arw` maps to `CompressionFamily::Arw6` | `fornax-arw` only |
| `32767` | Sony-private ARW1/ARW2 compression, not TIFF-registered; pinned dnglab/Rawler and LibRaw behavior in `sony-arw.md` | One-strip packed Sony stream; stored byte count distinguishes ARW2 | `fornax-arw` maps to `CompressionFamily::Arw1` or `Arw2` | `fornax-arw` only |
| `34892` | DNG 1.4+ Lossy JPEG; DNG 1.7.1 p.21 and Appendix A | DNG 8-bit `LinearRaw` or `PhotometricMask`; Fornax supports one 8-bit component | DNG maps to `DecodeCompression::LossyJpeg` | DNG override to `LossyJpeg` |
| `50000` | LibTIFF self-assigned Zstandard, explicitly not Adobe-registered; LibTIFF 4.0.10 release notes | Fornax integer or float raw payloads | DNG maps to `DecodeCompression::Zstd`; DNG writer emits `50000` experimentally | DNG compatibility override to `Zstd` |
| `50002` | GDAL self-assigned JPEG XL TIFF value, not Adobe-registered and superseded for writing by DNG `52546`; GDAL discussion and current LibTIFF header warning | Fornax single-channel 8-16-bit integer JPEG XL | DNG maps to `DecodeCompression::JpegXl` | DNG compatibility override to `JpegXl` |
| `52546` | DNG 1.7 JPEG XL; DNG 1.7.1 p.21 and Appendix A | DNG permits 8-16-bit integer, 16-bit float, 1 or 3 planes; Fornax supports one 8-16-bit integer plane | DNG maps to `DecodeCompression::JpegXl` | DNG override to `JpegXl` |

`fornax_tiff::resolve_tiff_compression` owns only TIFF defaults `1`, `7`, and
`8`. `fornax-dng` resolves `34892`, `50000`, `50002`, and `52546` first, then
falls back to that resolver. TIFF `7` baseline-DCT is outside the supported
single-band RAW subset. Sony-private IDs never enter either generic resolver.
The corresponding typed DNG values are owned by
`fornax_dng::constants::DngCompressionMethod`.
