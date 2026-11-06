# fornax-tiff

Generic TIFF container parsing, typed values, and classic-TIFF writing used by
Fornax RAW decoders.

## Container Support

| Feature | Read | Write |
| --- | --- | --- |
| Classic TIFF | Little- and big-endian | Little-endian |
| BigTIFF | Yes | No |
| Root IFD chain | Yes | One image directory |
| `SubIFDs` | Yes | No |
| Strips and tiles | Metadata model | Caller-defined offsets |
| Unknown well-formed tags | Preserved when the field type is understood | Typed values only |

The crate intentionally owns container mechanics only. DNG tag semantics and
raw codecs belong to `fornax-dng`.
