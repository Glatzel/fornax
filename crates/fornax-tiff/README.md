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

`TiffTag` names standard TIFF/TIFF-EP fields while `Tag` carries dynamic
identifiers. `CompressionMethod` and `PhotometricInterpretation` are typed
field constants in `fornax_tiff::constants`. `TiffDirectory` accumulates and
finalizes writer entries.

`RasterLayout` reads `ImageWidth`, `ImageLength`, strips/tiles, offsets, and
byte counts into a validated row-major chunk list. `RasterRequest` accepts an
already-resolved `fornax_codec::DecodeCompression`, validates every encoded
range against the source, and converts to a format-neutral codec request.
`resolve_tiff_compression` supplies the generic TIFF defaults for compression
IDs `1`, `7`, and `8`. DNG, self-assigned, and vendor-specific IDs are resolved
by their owning format crates before they construct a TIFF raster request.

`parse_ifd_at` reads a classic-TIFF IFD that is not linked through the root
chain. An offset correction maps out-of-line values stored relative to a larger
container into an extracted or decrypted sub-buffer; Sony private metadata uses
that path.

DNG raw compression identifiers currently represented by
`CompressionMethod` are:

| Identifier | Meaning | Status |
| ---: | --- | --- |
| `1` | Uncompressed | Recognized |
| `7` | Lossless JPEG-92 | Recognized |
| `8` | Deflate | Recognized |
| `34892` | Lossy JPEG | Recognized |
| `50000` | Experimental Zstandard | Recognized |
| `50002` | Legacy TIFF JPEG XL | Recognized |
| `52546` | DNG 1.7 JPEG XL | Recognized |

Compression `50002` is the legacy TIFF JPEG XL identifier and `52546` is the DNG
1.7 JPEG XL identifier. Other TIFF compression identifiers remain available
through `CompressionMethod::Unknown`; recognizing an identifier here does not by
itself imply that a codec can decode every payload layout.

Codec support is mandatory. Zstandard and future codecs are mandatory
dependencies without optional codec features.

The crate intentionally owns container mechanics only. DNG-only tag semantics
belong to `fornax-dng`; format-neutral raw codecs belong to `fornax-codec`.

## References

- [TIFF 6.0 Specification](https://www.adobe.io/open/standards/TIFF.html) — Adobe TIFF 6.0 specification.
- [BigTIFF Design](https://libtiff.gitlab.io/libtiff/specification/bigtiff.html) — BigTIFF extensions and 64-bit offsets.
