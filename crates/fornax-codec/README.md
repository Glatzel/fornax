# fornax-codec

Format-neutral raw sample codecs for Fornax decoder implementations.

`fornax-codec` owns uncompressed sample assembly, TIFF-compatible predictors,
lossless and lossy JPEG, JPEG XL, Deflate, and optional Zstandard handling. It
returns `fornax_core::RawMosaic` values and has no dependency on TIFF or DNG
container types. TIFF compression identifiers belong to
[`fornax-tiff`](../fornax-tiff/README.md), while
[`fornax-dng`](../fornax-dng/README.md) translates container metadata into
codec requests.

## Interfaces

`DecodeCompression` and `EncodeCompression` select the algorithm inside their
respective request types. `decode(&DecodeRequest)` and
`encode(&EncodeRequest)` dispatch directly to the selected implementation.
Decoding accepts the image geometry, chunk locations, sample layout, byte
order, predictor, and source bytes. Encoding returns row-major
`EncodedChunks`. Lossy JPEG and JPEG XL are absent from `EncodeCompression`, so
they cannot be selected for encoding.

`fornax_dng` translates TIFF/DNG tags into these requests. Container parsing,
format tags, raw transforms, crop metadata, and DNG writer policy remain in the
format crates.

## Compression Support

| TIFF compression | Decode | Encode |
| --- | --- | --- |
| Uncompressed (`1`) | Packed integers and floats | U8/U16/U32/F32 |
| Lossless JPEG-92 (`7`) | Single-sample strips/tiles, predictors 1-7 | U8/U16 tiles |
| Deflate (`8`) | Integer and float predictors | U8/U16/U32/F32 tiles |
| Lossy JPEG (`34892`) | Single-component 8-bit | No |
| JPEG XL (`52546`) | Single-channel 8-16-bit mosaics | No |
| Zstandard (`50000`, experimental) | Integer and float payloads | Integer and float tiles |

Unknown TIFF compression identifiers are reported as unsupported.

The DNG layer maps compression `52546` to the JPEG XL decoder. Compression
`50002` is the legacy TIFF JPEG XL identifier and is currently not routed by
`fornax-dng`.

## Current codec limitations

- Lossless JPEG-92 does not yet support pre-DNG-1.1 precision handling, restart
  intervals, Huffman selector `0x10`, or multi-component subsampled and
  half-height tile layouts.
- Lossy JPEG decoding accepts only one 8-bit component. Three-component
  `LinearRaw` payloads are outside the mono `RawMosaic` contract.
- JPEG XL decoding accepts only one 8-16-bit channel. Multi-channel `LinearRaw`
  payloads are outside the mono `RawMosaic` contract.
- Uncompressed, Deflate, and Zstandard decoding accept only one raw sample per
  pixel in the DNG decoder.

## Features

| Feature | Effect |
| --- | --- |
| Default | JPEG, JPEG XL, Deflate, and lossless JPEG-92 |
| `zstd` | Adds Zstandard decoding and encoding |

Use `EncodeCompression::Zstd { level }` to select a Zstandard encoding level.
Decoding uses `DecodeCompression::Zstd` because the level is stored in the
stream. Without `zstd`, both Zstandard directions return
`CodecError::Unsupported`.

The approved target is mandatory codec availability: zstd and every future
codec are compiled unconditionally. The current `zstd` feature and its fallback
errors remain a pending migration and must be removed before documenting zstd
as always available in shipped builds.
