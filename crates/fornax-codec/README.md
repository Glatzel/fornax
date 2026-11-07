# fornax-codec

Format-neutral raw sample codecs for Fornax decoder implementations.

`fornax-codec` owns uncompressed sample assembly, TIFF-compatible predictors,
lossless and lossy JPEG, JPEG XL, Deflate, and optional Zstandard handling. It
returns `fornax_core::RawMosaic` values and has no dependency on TIFF or DNG
container types.

## Interfaces

`Codec` selects a raw sample codec and exposes `decode` and `encode` methods.
Decoding accepts a `DecodeRequest` containing the image geometry, chunk
locations, sample layout, byte order, predictor, and source bytes. Encoding
accepts an `EncodeRequest` and returns row-major `EncodedChunks`. Lossy JPEG and
JPEG XL return `CodecError::Unsupported` when encoding is requested.

`fornax_dng` translates TIFF/DNG tags into these requests. Container parsing,
format tags, raw transforms, crop metadata, and DNG writer policy remain in the
format crates.

## Features

| Feature | Effect |
| --- | --- |
| Default | JPEG, JPEG XL, Deflate, and lossless JPEG-92 |
| `zstd` | Adds Zstandard decoding and encoding |

Use `Codec::Zstd { level }` to select a Zstandard encoding level. Without
`zstd`, both Zstandard directions return `CodecError::Unsupported`.
