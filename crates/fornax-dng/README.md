# fornax-dng

Pure-Rust DNG parsing, metadata conversion, raw codecs, transforms, and DNG
writing.

## Container and Layout Support

| Feature | Support |
| --- | --- |
| Classic TIFF and BigTIFF input | Yes |
| Full-resolution CFA IFD selection | Yes |
| Strips and tiles | Yes |
| Packed 1-32-bit integer samples | Yes |
| 16/24/32-bit float samples | Yes |
| Conventional 2x2 RGB Bayer handoff | Yes |
| X-Trans, four-color, monochrome, arbitrary CFA | Not supported |
| Auxiliary preview/depth/Exif persistence | Not supported |

## Compression Support

| TIFF compression | Decode | Encode |
| --- | --- | --- |
| Uncompressed (`1`) | Packed integers and floats | U8/U16/U32/F32 |
| Lossless JPEG-92 (`7`) | Strips/tiles, predictors 1-7 | U8/U16 tiles |
| Deflate (`8`) | Integer and float predictors | U32/F32 tiles |
| Lossy JPEG (`34892`) | Single-component 8-bit | No |
| JPEG XL (`52546`) | Single-channel 8-16-bit mosaics | No |

Unknown compression IDs are reported as unsupported. DNG output is regenerated
rather than byte-identical because source container bytes and auxiliary
directories are not retained.
