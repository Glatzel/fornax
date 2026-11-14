# Corpus Losslessness Classification

This note records why `compression_lossless` is `1`, `0`, or empty in the
decoder corpus baseline. The flag means that the implemented path reconstructs
stored integer codes without a known quantization or dithered loss step. It
does not mean that later camera-profile transforms are semantically lossless.

## Sources

- Adobe, *Digital Negative Specification 1.7.1.0*, compression definitions.
- TIFF 6.0 and the identifiers documented in
  [compression-ids.md](compression-ids.md).
- Rawler 0.8.0 at commit
  `62b77de1a206d467e910d55199191068b45a9cc1`, used as behavioral evidence for
  vendor bitstreams.
- Fornax's in-tree decoder references under
  [docs/reference/decoders/](../decoders/).

## Rules

| Format family | Lossless | Not lossless / not proven | Evidence |
| --- | --- | --- | --- |
| DNG/TIFF | Uncompressed, LZW, lossless JPEG, Deflate, PackBits, LZMA2, Zstandard | Lossy JPEG; JPEG XL and WebP are ambiguous | TIFF/DNG identifier semantics; the DNG path accepts lossless JPEG SOF3 |
| Sony ARW | ARW1, ARW6, lossless JPEG, uncompressed | ARW2 | ARW1 is a reversible difference stream; ARW2 quantizes 11-bit extrema into 7-bit deltas and applies Sony's deterministic dither in Rawler `decoders/arw.rs` |
| Nikon NEF | Uncompressed words, lossless trees, supported packed/word modes | Unsupported lossy/split modes are not emitted | Fornax exposes only modes accepted by its lossless Huffman/packed paths |
| Canon CR3 | CRX | Lossy CRX or unknown layouts are not emitted | The CRX parser accepts only the validated zero-level lossless layout |
| Hasselblad 3FR | Uncompressed, lossless JPEG | None among exposed paths | TIFF compression 1 and lossless JPEG-92 |
| Fujifilm RAF | Uncompressed packed/word, Fuji lossless | None among exposed paths | Accepted paths either unpack stored words or use the documented lossless decoder |
| Panasonic RW2 | V4, V5, V6, V8, unpacked/packed lossless control | None among exposed paths | Accepted decoder paths reconstruct integer codes without a quantization/dither stage |
| Phase One IIQ | Uncompressed, IIQ-L14, IIQ-L16 | Nonlinear IIQ-S and IIQ-Sv2 are conservatively excluded | Nonlinear curve mapping and Sv2 recovery/clamping do not prove exact reconstruction |

Empty means the file was not decoded or its codec could not be classified.
Ambiguous codecs remain ineligible for the golden corpus until stream-level mode
detection proves losslessness.
