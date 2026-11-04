# DNG and lossless JPEG-92 reference notes

## Authoritative sources

- [Adobe DNG Specification 1.7.1.0](https://download.adobe.com/pub/adobe/dng/DNG_Spec_1_7_1_0.pdf),
  September 2023. DNG raw-image selection, tag semantics, tile/strip layout,
  CFA coordinates relative to `ActiveArea`, and lossless JPEG-92 rules follow
  this specification.
- [TIFF 6.0 specification](https://www.adobe.io/content/dam/udp/en/open/standards/tiff/TIFF6.pdf).
  Classic TIFF headers, IFD entries, field types, inline values, and offset
  validation follow this document.
- [ITU-T T.81](https://www.itu.int/rec/T-REC-T.81/en), JPEG, Annex H. The
  lossless process uses the SOF3 frame marker, Huffman-coded differences,
  point transforms, and predictors 1 through 7 from Annex H.
- `external/dnglab` at
  `62b77de1a206d467e910d55199191068b45a9cc1`, especially
  `rawler/src/decoders/dng.rs`,
  `rawler/src/decompressors/ljpeg/`, and `rawler/src/ljpeg92.rs`. These were
  consulted as behavior checks for DNG raw-IFD selection, JPEG component
  interleaving, predictors, and tile assembly.

## Decisions supported by the sources

- A DNG may place a preview in the root IFD and the raw CFA image in a
  `SubIFD`. Selection therefore filters on `NewSubFileType`,
  `PhotometricInterpretation = 32803`, and geometry instead of taking the first
  image directory.
- DNG `CFAPattern` coordinates begin at `ActiveArea`, while Fornax carries a
  pattern relative to the full decoded mosaic. The handoff shifts the pattern
  by the active-area origin. For the accepted Canon fixture this changes the
  tag-order RGGB pattern into full-frame GBRG.
- A one-sample-per-pixel CFA may be represented by multiple interleaved JPEG
  components. The accepted fixture uses two 128-sample components per
  256-sample tile row. The entropy decoder writes those components back into
  one row-major sample stream.
- Predictors 1 through 7 use the left, above, and above-left neighbors across
  the interleaved component stream. The first pixel is initialized from the
  precision-dependent midpoint plus its Huffman difference.
- Byte-identical lossless export requires retaining the original TIFF
  container. Re-encoding samples loses tile boundaries, Huffman construction,
  metadata ordering, and non-raw preview directories.

## Known implementation differences

- The lossless-JPEG writer does not recompress arbitrary samples. A decoded
  lossless-JPEG source is copied byte for byte when lossless options are
  requested; uncompressed export is regenerated from samples.
- Restart markers, the pre-DNG-1.1 16-bit deviation, opcode lists,
  linearization tables, deflate, JPEG XL, lossy JPEG, and interleave factors
  are not implemented.
- Classic and BigTIFF headers and both byte orders are parsed, but only classic
  little-endian DNG is covered by the accepted integration fixture.
- Preview, thumbnail, mask, depth, and Exif image decoding are outside the raw
  decode path. Exif pointer tags are not copied to regenerated raw IFDs.

## License constraints

rawler is licensed LGPL-2.1. It was used as a behavioral reference only. The
implementation in `fornax-core` is independently written against the DNG,
TIFF, and JPEG specifications and does not add rawler to production
dependencies.
