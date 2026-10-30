# fornax

High-level façade for Fornax's decoding, preprocessing, demosaicing, and
post-processing stages, with Rawler, LibRaw, and Adobe DNG Converter decoder
backends behind Cargo features.

```bash
cargo run -p fornax --example pipeline -- path/to/image.cr2
```

The example runs the decoded RAW through preprocessing and linear demosaicing
and writes the intermediate images under `temp/pipeline`.
