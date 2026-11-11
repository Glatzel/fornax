# fornax-raf

Pure-Rust Fujifilm RAF container and CFA decoding for Fornax.

The crate parses RAF pointer blocks, the type-less Fujifilm proprietary tag
table, and the embedded Fuji TIFF/IFD. It decodes uncompressed 14/16-bit word
storage and Fujifilm block-lossless images for standard 2x2 Bayer and 6x6
X-Trans layouts, then maps source-supported geometry, CFA, levels, white
balance, camera identity, and crop metadata into `fornax_tiff::Directory`.

X-Trans `raw_mosaic` and `dng_metadata` accessors succeed so the mosaic can be
written as a CFA DNG. `process_metadata` and `cropped_mosaic` return typed
unsupported errors because the active preprocessing contract is 2x2 Bayer
only. Supported GFX Bayer models use explicit sensor profiles; unknown Bayer
models, lossy compression, SuperCCD rotation, DBP, and other legacy layouts
return typed unsupported errors.

Behavioral provenance and licensing constraints are recorded in
[Fujifilm RAF references](../../docs/references/fuji-raf.md).
