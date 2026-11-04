# fornax-decoder

Pure-Rust, DNG-centred RAW decoding for Fornax.

This crate is currently a compiling scaffold. It exposes the DNG-shaped data
model, image-buffer-backed mosaic data, a tiff-value metadata map, and direct
tag lookup. Construction, protocol conversion, and DNG writing are intentional
`todo!()` placeholders; examples touching those paths use `no_run`. It contains
no RAW decompression or DNG serialization yet.

The design and format implementation order are documented in
[`docs/decoder/architecture.md`](../../docs/decoder/architecture.md).
