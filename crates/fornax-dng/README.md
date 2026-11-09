# fornax-dng

Pure-Rust DNG parsing, metadata conversion, raw transforms, and DNG writing.
Format-neutral sample decoding and supported codec encoders live in
`fornax-codec`.

`fornax-codec` returns single-band `SampleBuffer` values. This crate adapts
those buffers to `fornax_core::RawMosaic` before raw transforms and adapts
`RawMosaic` inputs to `SampleBuffer` at the DNG writer boundary, preserving the
storage-native decoder and writer APIs.

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
| One raw sample per pixel | Yes |
| Multi-channel `LinearRaw` | Not supported |
| Conventional 2x2 RGB Bayer handoff | Yes |
| X-Trans, four-color, monochrome, arbitrary CFA | Not supported |
| Auxiliary preview/depth/Exif persistence | Not supported |

Compression capabilities are documented in
[`fornax-codec`](../fornax-codec/README.md#compression-support). DNG output is
regenerated rather than byte-identical because source container bytes and
auxiliary directories are not retained.

Compression `50002` is the legacy TIFF JPEG XL identifier and compression
`52546` is the DNG 1.7 JPEG XL identifier. Both map to the single-channel JPEG
XL decoder.

Zstandard is always compiled and is selected with `DngCompression::Zstd`.
Compression 50000 is the libtiff/GDAL de-facto extension, not an
Adobe-registered DNG compression code.

Strip/tile geometry and encoded chunk validation come from
`fornax_tiff::RasterLayout` and `fornax_tiff::RasterRequest`. TIFF defaults
`1`, `7`, and `8` resolve through `fornax_tiff::resolve_tiff_compression`; this
crate resolves DNG and compatibility IDs `34892`, `50000`, `50002`, and `52546`
before falling back. The TIFF raster request receives the resolved codec
compression and does not interpret DNG tags.

The decoder accepts one raw sample per pixel. Three-sample `LinearRaw` payloads
using compression `1`, `7`, or `34892` are rejected before codec dispatch and
are not evidence that the underlying compression codec is missing.

## DNG writer benchmarks

```bash
cargo bench -p fornax-dng --bench writer
```

`FORNAX_BENCH_DNG_COMPRESSION` selects `uncompressed`, `lossless-jpeg`,
`deflate`, and `zstd` cases. `FORNAX_BENCH_ZSTD_LEVEL` defaults to `3`.
Synthetic sizes reuse `FORNAX_BENCH_SIZE`. Local runs use deterministic
high-entropy noise by default. Set `FORNAX_BENCH_DNG_RAW` to an absolute DNG
path to benchmark that image's decoded mosaic instead. CI benchmarks both the
noise cases and the default RAW fixture.

The benchmark prints encoded bytes and the ratio to raw sample storage before
timing starts. Each Criterion `benchmark.json` records the exact output DNG
size in `throughput.Bytes`.

## References

- [Digital Negative (DNG) Specification](https://helpx.adobe.com/camera-raw/digital-negative.html) — Adobe DNG format specification.
