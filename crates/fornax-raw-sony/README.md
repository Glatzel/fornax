# fornax-raw-sony

Pure-Rust Sony ARW/NRW decoding for Fornax.

The crate parses Sony TIFF containers and private metadata, selects the
full-resolution CFA image, and decodes uncompressed, ARW1, ARW2, ARW6, and
single-component tiled lossless-JPEG payloads. It maps Sony levels, white
balance, geometry, and CFA data into a `fornax_tiff::Directory` and implements
the shared decoder interfaces used by the facade and CLI.

ARW1, ARW2, ARW6, Sony LJPEG, curve-derived white-level logic, and the LLVC3
table are implemented here as a crate-private `ICodecDecode` codec.

Reduced-resolution YCbCr images, multi-sample ARQ files, and unknown
compression families are rejected with a typed unsupported-layout error.
