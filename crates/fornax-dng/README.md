# fornax-dng

Pure-Rust DNG parsing, metadata conversion, raw transforms, and DNG writing.
Format-neutral sample decoding and supported codec encoders live in
`fornax-codec`.

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

Compression capabilities are documented in
[`fornax-codec`](../fornax-codec/README.md#compression-support). DNG output is
regenerated rather than byte-identical because source container bytes and
auxiliary directories are not retained.

Zstandard support is disabled by default. Enable the `zstd` feature and select
`DngCompression::Zstd` explicitly. Compression 50000 is the libtiff/GDAL
de-facto extension, not an Adobe-registered DNG compression code.

## DNG writer benchmarks

```bash
cargo bench -p fornax-dng --features zstd --bench writer
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
