# fornax-nef

Pure-Rust Nikon NEF container and CFA decoding for Fornax.

The crate parses Nikon type-2 maker notes, selects the full-resolution CFA
strip IFD, decodes standard uncompressed, packed, and mode-3 lossless Huffman
modes `2`, `3`, `5`, `6`, `7`, `9`, and `10` at 12 and 14 bits, and maps
geometry, CFA, levels, camera identity, and white balance into
`fornax_tiff::Directory` and `ProcessMetadata`.

Lossy Huffman modes 1 and 4 are deferred by policy. Lossy-deferred, HE/HE*,
small raw mode 8, reduced-linear output, Coolpix-specific packing, multi-strip
mode 3, per-model geometry overrides, headerless maker notes, and non-2x2 CFA
layouts return typed unsupported errors.
