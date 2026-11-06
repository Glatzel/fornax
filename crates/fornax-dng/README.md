# fornax-dng

Pure-Rust DNG parsing, metadata conversion, raw codecs, transforms, and DNG
writing.

Decoded metadata is a `fornax_tiff::Directory`, keyed by the dynamic `Tag`
type. `DngTag` names DNG-only fields and converts into those keys; generic TIFF
fields and constants remain in `fornax-tiff`.

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
| Deflate (`8`) | Integer and float predictors | U8/U16/U32/F32 tiles |
| Lossy JPEG (`34892`) | Single-component 8-bit | No |
| JPEG XL (`52546`) | Single-channel 8-16-bit mosaics | No |
| Zstandard (`50000`, experimental) | Integer and float payloads | Integer and float tiles |

Unknown compression IDs are reported as unsupported. DNG output is regenerated
rather than byte-identical because source container bytes and auxiliary
directories are not retained.

Zstandard support is disabled by default. Enable the `zstd` feature and select
`DngCompression::Zstd` explicitly. Compression 50000 is the libtiff/GDAL
de-facto extension, not an Adobe-registered DNG compression code.

## DNG writer benchmarks

```bash
cargo bench -p fornax-dng --features zstd --bench writer
```

`FORNAX_BENCH_DNG_COMPRESSION` selects `uncompressed`, `lossless-jpeg`,
`deflate`, and `zstd` cases. `FORNAX_BENCH_ZSTD_LEVEL` defaults to `3`.
Synthetic sizes reuse `FORNAX_BENCH_SIZE`. The benchmark prints encoded bytes
and the ratio to raw U16 storage before timing starts.
