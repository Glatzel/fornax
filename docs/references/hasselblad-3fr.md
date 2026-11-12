# Hasselblad 3FR references

## Sources

- dnglab/Rawler commit
  `62b77de1a206d467e910d55199191068b45a9cc1`, LGPL-2.1:
  `rawler/src/decoders/tfr.rs`,
  `rawler/src/decompressors/ljpeg/mod.rs`,
  `rawler/src/decompressors/ljpeg/decompressors.rs`,
  `rawler/src/decompressors/ljpeg/huffman.rs`, and
  `rawler/data/cameras/hasselblad/*.toml`.
- LibRaw commit `cf02d66eb5c19d4e46df7aee0582e47a4918509a`, LGPL-2.1 or
  CDDL-1.0: `src/decoders/load_mfbacks.cpp`,
  `src/metadata/hasselblad_model.cpp`, and `src/metadata/tiff.cpp`.

The pinned sources support these observed behaviors:

- 3FR raw pixels are selected from a SubIFD with `WhiteLevel`; root IFDs are
  previews.
- Uncompressed X-series and CFV-100C data is little-endian 16-bit word storage.
- Hasselblad lossless JPEG uses predictor 8 with two persistent row predictors,
  two leading Huffman lengths per pair, and a `0x8000` pair reset per row.
- Rawler's predictor-8 path reads 3FR entropy through `BitPumpMSB32`, which
  reverses byte order within each four-byte refill and does not apply JPEG
  marker unstuffing.
- Rawler enables its Huffman `dng_bug` mode for 3FR: difference class 16
  consumes 16 stored bits and represents `-32768`.
- Rawler's camera profiles provide RGGB and X1D-family active borders because
  the accepted raw IFDs do not contain CFA tags.

## Implementation boundary

Fornax independently implements TIFF selection, metadata mapping, DNG export,
and predictor-8 sample decoding from these behavioral observations. Rawler and
LibRaw are development-only references. No Rawler or LibRaw source, comments,
or naming is copied into production crates.

Rawler's high-level decoder is not used as the fixture oracle because its
0.8.0 camera database rejects one accepted CFV-50c model. Tests instead use its
public low-level predictor-8 decompressor and direct 16-bit word reads, which
isolate exact sample parity from camera-name coverage.

FFF, merged/multi-shot, tiled, unknown profile, and unknown compression layouts
remain outside the implemented tier and return typed unsupported errors.
