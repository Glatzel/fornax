# DNG decoder reference notes

This note is the canonical detailed reference for Fornax's DNG decoder scope,
storage behavior, metadata mapping, and acceptance boundary.

## Support summary

Tier 0 accepts classic TIFF, BigTIFF, strips/tiles, single-channel CFA or
`LinearRaw` image selection, uncompressed integer/float samples, lossless
JPEG-92 predictors 1-7, Deflate, single-component lossy JPEG, JPEG XL, and
feature-gated Zstandard. Conventional 2x2 RGB Bayer data crosses the
preprocessing boundary.

Multi-channel `LinearRaw`, non-2x2 interleave, unsupported lossless-JPEG tile
variants, downstream opcodes, and auxiliary-directory regeneration remain
outside this slice. Export is regenerated rather than byte-identical.

## Sources

- Adobe Digital Negative Specification 1.7.1.0, September 2023.
- [DNG compression references](dng-compression.md).
- [DNG and lossless JPEG-92 references](dng-lossless-jpeg.md).
- [DNG opcode references](dng-opcodes.md).
- TIFF 6.0 and TIFF/EP for directory, strip, tile, integer, and float storage.

Rawler and LibRaw are LGPL development references only. Production code is
independently written from specifications and observed behavior.

## Implemented scope

- Classic TIFF in either byte order, classic BigTIFF, root IFD chains, and
  `SubIFDs`.
- Largest non-reduced CFA or `LinearRaw` image selection; previews,
  thumbnails, masks, depth images, and Exif images are never raw sources.
- Uncompressed packed 1-32-bit integer and 16/24/32-bit floating-point
  strips/tiles.
- Lossless JPEG-92 predictors 1-7 at 2-16 bits for accepted sample layouts.
- Deflate, single-component lossy JPEG, JPEG XL, and experimental Zstandard
  chunks.
- 2x2 interleave, linearization, `OpcodeList1`, checked geometry, parallel
  strip/tile decode, and active-area Bayer handoff.
- Uncompressed DNG output and tiled lossless JPEG-92 or Deflate output; export
  is regenerated rather than byte-identical.

The accepted `colorchart-5D2-6000K.dng` fixture decodes to `5792x3804`, crops
to `5634x3753`, changes full-frame GBRG to active-area RGGB, and reports a
2x2 black-level pattern of 1024 and white level 15600.

## Metadata

The decoder retains typed `fornax_tiff::Directory` values, including unknown
well-formed tags. Required processing fields include `ActiveArea`,
`DefaultCropOrigin`, `DefaultCropSize`, CFA repeat/pattern/planes/layout,
black-level patterns and deltas, `WhiteLevel`, white balance, calibration,
make/model, and serial identity.

Missing `ActiveArea` defaults to the decoded frame. Missing default crop
defaults to the active area. Missing `CFAPlaneColor` defaults to RGB plane IDs.
`OpcodeList1` and `LinearizationTable` affect the raw mosaic; `OpcodeList2` and
`OpcodeList3` remain metadata for later stages.

## CFA boundary

Only conventional 2x2 RGB Bayer layouts (`RGGB`, `BGGR`, `GRBG`, `GBRG`) cross
the preprocessing boundary. Other repeat sizes, plane colors, layouts, and
patterns return typed unsupported errors. Multi-channel `LinearRaw` remains
outside the single-mosaic decoder contract.

## Known limits

- Pre-DNG-1.1 lossless-JPEG compatibility, restart intervals, selector
  `0x10`, and multi-component subsampled tiles.
- Multi-channel `LinearRaw`, `SubTileBlockSize`, and non-2x2 interleave.
- Geometric/front-end opcodes and downstream `OpcodeList2`/`OpcodeList3`.
- Automatic BigTIFF output for regenerated files beyond classic TIFF limits.
- Preview, thumbnail, mask, depth, and nested Exif/GPS regeneration.

## Validation boundary

Synthetic tests cover both byte orders, BigTIFF, strips/tiles, accepted integer
and float storage, every supported compression, CFA round-trip, malformed
metadata, and unsupported handoff. The manual corpus covers Canon 5D Mark III
DNG variants, Nikon D80, Sigma fp, Ricoh GR III, Sony ILCE-1M2, and Nikon
Z 7 II. Accepted decoded samples match Rawler after active-area alignment.
