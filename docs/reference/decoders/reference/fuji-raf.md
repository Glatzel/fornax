# Fujifilm RAF references

This reference note records pinned sources, decoder behavior, and implementation boundaries for Fujifilm RAF.

## Sources

- Rawler `0.8.0`, LGPL-2.1:
  `src/decoders/raf.rs`, `src/decoders/raf/fuji_decompressor.rs`,
  `src/decoders/raf/dbp.rs`, and `data/cameras/fuji/*.toml`.
- LibRaw commit `cf02d66eb5c19d4e46df7aee0582e47a4918509a`, LGPL-2.1 or
  CDDL-1.0: `src/metadata/fuji.cpp`,
  `src/decoders/fuji_compressed.cpp`, and the SuperCCD decoding paths.

The source revisions support the following observed behavior:

- RAF offsets 92 and 100 address the type-less RAF tag table and the embedded
  Fuji TIFF/IFD respectively; offset 84 addresses the embedded JPEG preview.
- The Fuji IFD uses tags `0xF000`-`0xF010`. The raw strip offset is relative
  to the embedded TIFF base.
- Fujifilm block compression starts with the `IS` signature and stores block
  sizes after a 16-byte header. The block-size table is padded to a 16-byte
  boundary. Lossless compression sets the flag at byte 2 to one.
- X-Trans layout tag `0x0131` stores 36 CFA planes in reverse DNG order.
- Modern GFX 50/100 cameras require explicit 2x2 RGGB and active-area
  profiles; the RAF crop tags provide the default crop.

## Implementation boundary

Fornax's codec and container implementation is independently written from
these behavioral observations. No Rawler or LibRaw implementation source,
comments, or naming is copied into production crates. Rawler is used only as a
development-only fixture oracle in tests.

Lossy block compression, SuperCCD rotation/staggering, and FinePix DBP are
outside the implemented tier and return typed unsupported errors.
