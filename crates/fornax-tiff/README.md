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
| Strips and tiles | Metadata model | Caller-defined offsets |
| Unknown well-formed tags | Preserved when the field type is understood | Typed values only |

`TiffTag` names standard TIFF/TIFF-EP fields while `Tag` carries dynamic
identifiers. `CompressionMethod` and `PhotometricInterpretation` are typed
field constants in `fornax_tiff::constants`. `TiffDirectory` accumulates and
finalizes writer entries.

DNG raw compression identifiers currently represented by
`CompressionMethod` are:

| Identifier | Meaning | Status |
| ---: | --- | --- |
| `1` | Uncompressed | Recognized |
| `7` | Lossless JPEG-92 | Recognized |
| `8` | Deflate | Recognized |
| `34892` | Lossy JPEG | Recognized |
| `50000` | Experimental Zstandard | Recognized |
| `52546` | DNG 1.7 JPEG XL | Recognized |

Compression `50002` is the legacy TIFF JPEG XL identifier and is not yet
represented by `CompressionMethod`. Other TIFF compression identifiers remain
available through `CompressionMethod::Unknown`; recognizing an identifier here
does not by itself imply that a codec can decode every payload layout.

Codec support is intended to be mandatory. Zstandard and future codecs must not
introduce optional codec features once the current feature-gated implementation
is migrated.

The crate intentionally owns container mechanics only. DNG-only tag semantics
belong to `fornax-dng`; format-neutral raw codecs belong to `fornax-codec`.

## References

- [TIFF 6.0 Specification](https://www.adobe.io/open/standards/TIFF.html) — Adobe TIFF 6.0 specification.
- [BigTIFF Design](https://libtiff.gitlab.io/libtiff/specification/bigtiff.html) — BigTIFF extensions and 64-bit offsets.
