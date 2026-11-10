# TIFF-family compression ID ownership

This audit records the ownership used to split generic TIFF vocabulary from
DNG-specific vocabulary. Codec availability is tracked separately: an
unimplemented or disabled codec does not change who owns an identifier.

## Sources

- Adobe, *TIFF 6.0 Specification*, final revision dated 3 June 1992:
  <https://download.osgeo.org/libtiff/doc/TIFF6.pdf>.
- Adobe, *Draft TIFF Technical Note #2*, 17 March 1995:
  <https://libtiff.gitlab.io/libtiff/specification/technote2.html>.
- ISO, *ISO 12234-2:2001*, TIFF/EP image data format, 15 October 2001:
  <https://www.iso.org/standard/29377.html>.
- Adobe, *Digital Negative Specification 1.7.1.0*, September 2023:
  <https://download.adobe.com/pub/adobe/dng/DNG_Spec_1_7_1_0.pdf>.
- LibTIFF 4.0.0 release notes:
  <https://libtiff.gitlab.io/libtiff/releases/v4.0.0.html>.
- LibTIFF 4.0.10 release notes:
  <https://libtiff.gitlab.io/libtiff/releases/v4.0.10.html>.
- LibTIFF 4.7.0 pinned `tiff.h`:
  <https://gitlab.com/libtiff/libtiff/-/raw/v4.7.0/libtiff/tiff.h>.
- GDAL maintainer discussion on JPEG XL identifier `50002`, 6 January 2025:
  <https://www.mail-archive.com/gdal-dev@lists.osgeo.org/msg42055.html>.
- LibTIFF merge request for DNG JPEG XL identifier `52546`, merged 29 October
  2024: <https://gitlab.com/libtiff/libtiff/-/merge_requests/672>.

## Ownership table

| ID | Meaning | Defining source and version | DNG-defined? | Fornax owner |
| ---: | --- | --- | --- | --- |
| `1` | Uncompressed, byte-aligned packed data | TIFF 6.0 (3 June 1992), section 4 and Compression field | No | `fornax-tiff`; raster resolver to `Uncompressed` |
| `5` | LZW adaptive lossless compression | TIFF 6.0 (3 June 1992), section 13 | No | `fornax-tiff` vocabulary; known codec unavailable until an `lzw` feature exists |
| `7` | JPEG stream, including sequential lossy or lossless processes | TIFF Technical Note #2 (17 March 1995); TIFF/EP 1.0 section 4.6.1 uses it for baseline JPEG | No | `fornax-tiff` vocabulary and current lossless-JPEG raw subset |
| `8` | Adobe Deflate/ZIP compression | Adobe TIFF-family assignment, identified as Adobe Deflate by LibTIFF 4.7.0; DNG 1.7.1.0 adopts it in DNG 1.4.0.0 | No | `fornax-tiff`; raster resolver to `Deflate` |
| `32773` | PackBits byte-oriented run-length compression | TIFF 6.0 (3 June 1992), section 9 | No | `fornax-tiff` vocabulary; known codec unavailable until a `packbits` feature exists |
| `32946` | Legacy LibTIFF Deflate/ZIP identifier | LibTIFF 4.7.0 `tiff.h`; older experimental LibTIFF documentation | No | `fornax-tiff` vocabulary; raster resolver to `Deflate` |
| `34892` | DNG lossy JPEG | DNG 1.7.1.0 pages 20-21; introduced by DNG 1.4.0.0 | Yes | `fornax-dng`; DNG override to `LossyJpeg` |
| `34925` | LZMA2-compressed data | LibTIFF 4.0.0 release notes added tag `34925`; LibTIFF 4.7.0 `tiff.h` names it LZMA2 | No | `fornax-tiff` vocabulary; known codec unavailable until an `lzma` feature exists |
| `50000` | Zstandard-compressed data | LibTIFF 4.0.10 self-assigned `50000`, explicitly not Adobe-registered | No | `fornax-tiff` vocabulary; resolver to `Zstd` when `zstd` is enabled |
| `50001` | WebP-compressed data | LibTIFF 4.0.10 self-assigned `50001`, explicitly not Adobe-registered | No | `fornax-tiff` vocabulary; known codec unavailable until a `webp` feature exists |
| `50002` | JPEG XL-compressed data | GDAL self-assigned `50002` for TIFF JPEG XL; LibTIFF 4.7.0 recognizes it and marks it unregistered | No | `fornax-tiff` vocabulary; resolver to `JpegXl` when `jpegxl` is enabled |
| `52546` | DNG JPEG XL | DNG 1.7.1.0 pages 20-21; introduced by DNG 1.7.0.0 | Yes | `fornax-dng`; DNG override to `JpegXl` |

## Decisions

- `34892` and `52546` are the only requested IDs defined by DNG and therefore
  remain in `DngCompressionMethod`.
- `50000`, `50001`, and `50002` are external TIFF-family self-assignments, not
  Adobe-defined DNG values.
- `8` and `32946` have distinct registration histories but the same Deflate
  payload meaning. They are not contradictory ownership claims.
- Sony-private compression IDs remain owned by `fornax-arw` and are outside the
  generic TIFF resolver. See [sony-arw.md](sony-arw.md).
- The resolver must report undocumented IDs as unknown and documented IDs
  without available codecs as unsupported with the relevant feature name.
