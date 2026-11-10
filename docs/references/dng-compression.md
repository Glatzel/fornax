# DNG compression references

## Sources

- Adobe, *Digital Negative (DNG) Specification 1.7.1.0*, September 2023:
  <https://download.adobe.com/pub/adobe/dng/DNG_Spec_1_7_1_0.pdf>.
- TIFF-family ownership evidence:
  [compression-ids.md](compression-ids.md).
- RFC 8878, *Zstandard Compression and the `application/zstd` Media Type*:
  <https://www.rfc-editor.org/rfc/rfc8878>.
- Sony ARW/NRW behavior and pinned vendor-reference notes:
  [sony-arw.md](sony-arw.md).

## Decisions

Fornax recognizes DNG compression 34892 (lossy JPEG) and 52546 (JPEG XL) as
DNG-owned values. Unknown DNG values remain representable in
`DngCompressionMethod::Unknown` and produce a typed decode error.

The generic TIFF resolver owns compression 1 (uncompressed), 5 (LZW), 7
(JPEG), 8 and 32946 (Deflate), 32773 (PackBits), 34925 (LZMA2), 50000
(Zstandard), 50001 (WebP), and 50002 (JPEG XL). DNG resolves its two owned IDs
first and then calls the TIFF resolver.

Compression 50000, 50001, and 50002 are external TIFF-family
self-assignments, not Adobe-defined DNG compression values. Zstandard support
remains experimental in Fornax.

Codec availability follows Cargo features. A known TIFF ID whose codec feature
is unavailable returns `TiffCompressionError::UnsupportedCompression` and names
the required feature. An undocumented ID returns
`TiffCompressionError::UnknownCompression`. This distinction prevents codec
configuration from being mistaken for an unknown format.

Uncompressed, lossless JPEG-92, and Deflate remain available in the TIFF raster
path. JPEG XL and Zstandard are feature-gated. LZW, PackBits, LZMA, and WebP are
recognized vocabulary without implementations in this step.

The production DNG path must not add `zstd-sys` or another C-backed codec.
LibRaw, darktable, and RawTherapee remain development-only comparison
references.

Fornax uses `structured-zstd`, a pure-Rust encoder/decoder. Each DNG tile is an
independent Zstandard frame so tile decoding remains parallel. The default
writer level is 3; LosslessJPEG remains the default interoperable compression.

## Compression ownership

The complete source-versioned ownership table is maintained in
[compression-ids.md](compression-ids.md). Sony-private IDs never enter either
generic resolver and remain owned by `fornax-arw`.
