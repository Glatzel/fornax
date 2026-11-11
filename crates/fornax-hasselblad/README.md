# fornax-hasselblad

Pure-Rust Hasselblad 3FR container and CFA decoding for Fornax.

The crate selects the full-resolution CFA SubIFD from the TIFF container and
decodes the accepted uncompressed 16-bit little-endian and predictor-8
lossless-JPEG layouts. Source geometry, crop, levels, white balance, color
matrix, identity, orientation, and safe EXIF metadata are mapped into
`fornax_tiff::Directory` for DNG export.

Accepted camera profiles use a conventional 2x2 RGGB CFA. X1D-family profiles
also remove the inactive sensor borders. Unknown tagless models, non-2x2 CFA
handoffs, FFF, merged/multi-shot, tiled, and unknown compression layouts return
typed unsupported errors.
