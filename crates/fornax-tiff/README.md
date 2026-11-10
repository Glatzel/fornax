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
`resolve_tiff_compression` supplies the generic TIFF defaults for compression
IDs `1`, `7`, and `8`. DNG, self-assigned, and vendor-specific IDs are resolved
by their owning format crates before they construct a TIFF raster request.

`parse_ifd_at` reads a classic-TIFF IFD that is not linked through the root
chain. An offset correction maps out-of-line values stored relative to a larger
container into an extracted or decrypted sub-buffer; Sony private metadata uses
that path.

## Raster Feature

The default feature set provides TIFF tags, field constants, values, parsing,
and writing without pulling in a raw codec implementation. Enable the `raster`
feature to add `RasterLayout`, `RasterRequest`, `RasterStorage`, `RasterError`,
and `resolve_tiff_compression`. This feature enables the optional
`fornax-codec` dependency and its codec, image, and Rayon dependency stack.

TIFF-family compression identifiers represented by `CompressionMethod` are:

| Identifier | Meaning | Status |
| ---: | --- | --- |
| `1` | Uncompressed | Recognized |
| `7` | Lossless JPEG-92 | Recognized |
| `8` | Deflate | Recognized |

DNG compression `34892`, `52546`, and compatibility identifiers `50000` and
`50002` are owned by `fornax-dng`. Other TIFF compression identifiers remain
available through `CompressionMethod::Unknown`; recognizing an identifier here
does not by itself imply that a codec can decode every payload layout.

When the `raster` feature is enabled, codec support is mandatory. Zstandard and
future codecs are mandatory dependencies without optional codec features.

The crate intentionally owns container mechanics only. DNG-only tag semantics
belong to `fornax-dng`; format-neutral raw codecs belong to `fornax-codec`.

## References

- [TIFF 6.0 Specification](https://www.adobe.io/open/standards/TIFF.html) — Adobe TIFF 6.0 specification.
- [BigTIFF Design](https://libtiff.gitlab.io/libtiff/specification/bigtiff.html) — BigTIFF extensions and 64-bit offsets.
