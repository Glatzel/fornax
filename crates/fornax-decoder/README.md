# fornax-decoder

Pure-Rust, DNG-centred RAW decoding for Fornax.

This crate is currently a compiling scaffold. It exposes the DNG-shaped data
model, typed tag storage, CFA-aware handoff checks, and unsupported decode and
write stubs. It contains no RAW decompression or DNG serialization yet.

The design and format implementation order are documented in
[`docs/decoder/architecture.md`](../../docs/decoder/architecture.md).
