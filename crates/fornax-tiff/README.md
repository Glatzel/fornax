# fornax-tiff

Generic TIFF tags, field constants, container parsing, typed values, and
classic-TIFF writing used by Fornax RAW decoders.

## Container Support

| Feature | Read | Write |
| --- | --- | --- |
| Classic TIFF | Little- and big-endian | Little-endian |
| BigTIFF | Yes | No |
| Root IFD chain | Yes | One image directory |
| `SubIFDs` | Yes | No |
| Inner IFDs at absolute offsets with optional offset correction | Yes | No |
| Strips and tiles | Validated raster layout | Caller-defined offsets |
| Unknown well-formed tags | Preserved when the field type is understood | Typed values only |

`TiffTag` names TIFF 6.0 and TIFF/EP fields while `Tag` carries dynamic
identifiers. `CompressionMethod` and `PhotometricInterpretation` are typed
TIFF-family field constants in `fornax_tiff::constants`; DNG-specific values
are owned by `fornax-dng`. `TiffDirectory` accumulates and finalizes writer
entries.

`RasterLayout` reads `ImageWidth`, `ImageLength`, strips/tiles, offsets, and
byte counts into a validated row-major chunk list. `RasterRequest` accepts an
already-resolved `fornax_codec::DecodeCompression`, validates every encoded
range against the source, and converts to a format-neutral codec request.
`resolve_tiff_compression` supplies the generic TIFF-family mapping and
distinguishes unknown IDs, implemented codecs whose feature is disabled, and
known IDs without a Fornax implementation. DNG IDs and Sony-private IDs are
resolved by their owning format crates before they construct a TIFF raster
request.

`parse_ifd_at` reads a classic-TIFF IFD that is not linked through the root
chain. An offset correction maps out-of-line values stored relative to a larger
container into an extracted or decrypted sub-buffer; Sony private metadata uses
that path.

## Raster Feature

The default feature set includes `lzw` and `packbits`, standard TIFF
compression codecs, and therefore also enables `raster`. Use
`--no-default-features` for TIFF tags, field constants, values, parsing, and
writing without a raw codec implementation. Enable `raster` alone to add
`RasterLayout`, `RasterRequest`, `RasterStorage`, `RasterError`, and
`resolve_tiff_compression` with only uncompressed, lossless JPEG-92, and
Deflate support. Any codec feature enables the optional `fornax-codec`
dependency and its codec, image, and Rayon dependency stack.

TIFF-family compression identifiers represented by `CompressionMethod` are:

| Identifier | Meaning | Raster resolver status |
| ---: | --- | --- |
| `1` | Uncompressed | Resolves to `Uncompressed` |
| `5` | LZW | Resolves to `Lzw` with `lzw` enabled |
| `7` | JPEG | Resolves to `LosslessJpeg` for the supported mono raw subset |
| `8` | Adobe Deflate | Resolves to `Deflate` |
| `32773` | PackBits | Resolves to `PackBits` with `packbits` enabled |
| `32946` | Legacy Deflate | Resolves to `Deflate` |
| `34925` | LZMA2 | Known; requires the future `lzma` feature |
| `50000` | Zstandard | Resolves to `Zstd` with `zstd` enabled |
| `50001` | WebP | Known; requires the future `webp` feature |
| `50002` | JPEG XL | Resolves to `JpegXl` with `jpegxl` enabled |

DNG compression `34892` and `52546` are owned by `fornax-dng`. Other TIFF
compression identifiers remain available through `CompressionMethod::Unknown`;
recognizing an identifier here does not imply that a codec can decode every
payload layout.

`raster` provides uncompressed, lossless JPEG-92, and Deflate support. `lzw`,
`packbits`, `jpegxl`, and `zstd` enable their matching optional codecs, with
`lzw` and `packbits` on by default. The source ownership and version evidence
is recorded in
[`compression-ids.md`](../../docs/reference/codecs/compression-ids.md).

The crate intentionally owns container mechanics only. DNG-only tag semantics
belong to `fornax-dng`; format-neutral raw codecs belong to `fornax-codec`.

## References

- [TIFF 6.0 Specification](https://www.adobe.io/open/standards/TIFF.html) — Adobe TIFF 6.0 specification.
- [BigTIFF Design](https://libtiff.gitlab.io/libtiff/specification/bigtiff.html) — BigTIFF extensions and 64-bit offsets.
