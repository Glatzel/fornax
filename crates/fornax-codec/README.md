# fornax-codec

Format-neutral raw sample codecs for Fornax decoder implementations.

`fornax-codec` owns uncompressed sample assembly, TIFF-compatible predictors,
lossless and lossy JPEG, JPEG XL, Deflate, and optional Zstandard handling. It
returns `fornax_core::RawMosaic` values and has no dependency on TIFF or DNG
container types.

## Interfaces

`ICodecDecode` accepts a `DecodeRequest` containing the image geometry, chunk
locations, sample layout, byte order, predictor, and source bytes. `ICodecEncode`
accepts an `EncodeRequest` and returns row-major `EncodedChunks`. The traits are
separate so decode-only JPEG and JPEG XL codecs do not expose meaningless encode
operations.

`fornax_dng` translates TIFF/DNG tags into these requests. Container parsing,
format tags, raw transforms, crop metadata, and DNG writer policy remain in the
format crates.

## Features

| Feature | Effect |
| --- | --- |
| Default | JPEG, JPEG XL, Deflate, and lossless JPEG-92 |
| `zstd` | Adds Zstandard decoding and encoding |

Without `zstd`, dispatching `Compression::Zstd` returns
`CodecError::Unsupported`.
