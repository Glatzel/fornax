# fornax

High-level façade for Fornax's decoding, preprocessing, demosaicing, and
post-processing stages. The current decoder accepts classic TIFF DNG files and
hands conventional 2x2 RGB Bayer data to the shared pipeline.

```bash
cargo run -p fornax --example pipeline -- path/to/image.dng
```

The example runs the decoded RAW through preprocessing and linear demosaicing
and writes the intermediate images under `temp/pipeline`.
