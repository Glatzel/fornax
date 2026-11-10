# fornax

High-level façade for Fornax's decoding, preprocessing, demosaicing, and
post-processing stages. The current decoder accepts DNG plus supported Sony
ARW/NRW files and hands conventional 2x2 RGB Bayer data to the shared pipeline.

```bash
cargo run -p fornax-cli -- --format jpeg path/to/image.dng
```

The converter runs the decoded RAW through preprocessing, demosaicing, and the
optional camera matrix before writing TIFF or JPEG output. Use
`fornax-cli --help` for configuration and output options.
