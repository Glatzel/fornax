# fornax-decoder

Pure-Rust, DNG-centred RAW decoding for Fornax.

The current implementation exposes the DNG-shaped data model,
image-buffer-backed mosaic data, a typed metadata map, and direct tag lookup.
It reads classic TIFF DNG files with uncompressed or lossless JPEG-92 strips and
tiles, validates conventional 2x2 RGB Bayer metadata for the shared pipeline,
and decodes the full-resolution CFA image instead of a preview.

Default lossless-JPEG export preserves the original container byte for byte.
Uncompressed export regenerates the raw CFA directory from decoded samples and
preserves required typed metadata.

The design and format implementation order are documented in
[`docs/decoder/architecture.md`](../../docs/decoder/architecture.md).
DNG and lossless-JPEG implementation decisions are recorded in
[`docs/references/dng-lossless-jpeg.md`](../../docs/references/dng-lossless-jpeg.md).
