# fornax-nef

Pure-Rust Nikon NEF container and CFA decoding for Fornax.

The crate parses Nikon type-2 maker notes, selects the full-resolution CFA
strip IFD, decodes standard uncompressed and packed modes `2`, `5`, `6`, `7`,
`9`, and `10`, and maps geometry, CFA, levels, camera identity, and white
balance into `fornax_tiff::Directory` and `ProcessMetadata`.

Huffman lossless/lossy NEF, HE/HE*, small raw mode 8, reduced-linear output,
Coolpix-specific packing, per-model geometry overrides, headerless maker
notes, and non-2x2 CFA layouts return typed unsupported errors.
