# fornax-codec

Format-neutral raw sample codecs for Fornax decoder implementations.

`fornax-codec` owns format-neutral requests and sample buffers, uncompressed
sample assembly, TIFF-compatible predictors, lossless and lossy JPEG, JPEG XL,
LZW, PackBits, Deflate, and optional Zstandard handling. It also provides
reusable canonical Huffman and MSB-first bit-reading primitives. It returns
`SampleBuffer` values and has no dependency on TIFF, DNG, or raw workflow
types. Vendor stream implementations live in their vendor decoder crates and
implement `ICodecDecode` there. TIFF compression identifiers belong to
[`fornax-tiff`](../fornax-tiff/README.md), while
[`fornax-dng`](../fornax-dng/README.md) translates container metadata into
codec requests.

## Interfaces

`DecodeCompression` implements `ICodecDecode`, so callers write
`codec.decode(&request)`. `DecodeRequest` contains no algorithm field. The
generic compression enum covers Uncompressed, Lossless JPEG, Deflate, LZW,
PackBits, Lossy JPEG, JPEG XL, and Zstandard. `EncodeCompression` continues to
select the encoder inside `EncodeRequest`, and `encode(&EncodeRequest)` returns
row-major `EncodedChunks`. The buffer stores
U8, U16, U32, F32, or F64 samples in an `image::ImageBuffer<Luma<T>, Vec<T>>`.
`Luma` denotes one scalar channel rather than necessarily luminance.
Decoding accepts the image geometry, chunk locations, sample layout, byte
order, predictor, and source bytes. Encoding returns row-major
Lossy JPEG and JPEG XL are absent from `EncodeCompression`, so
they cannot be selected for encoding.

`fornax_dng` translates TIFF/DNG tags into these requests. Container parsing,
format tags, raw transforms, crop metadata, and DNG writer policy remain in the
format crates.

## Compression Support

| TIFF compression | Decode | Encode |
| --- | --- | --- |
| Uncompressed (`1`) | Packed integers and floats | U8/U16/U32/F32/F64 |
| Lossless JPEG-92 (`7`) | Single-sample strips/tiles, predictors 1-7 | U8/U16 tiles |
| LZW (`5`) | TIFF MSB-first words and size switches | U8/U16/U32/F32/F64 tiles |
| Deflate (`8`) | Integer and float predictors | U8/U16/U32/F32/F64 tiles |
| PackBits (`32773`) | Literal, repeat, and no-op controls | U8/U16/U32/F32/F64 tiles |
| Lossy JPEG (`34892`) | Single-component 8-bit | No |
| JPEG XL (`50002`, `52546`) | Single-channel 8-16-bit mosaics | No |
| Zstandard (`50000`, experimental) | Integer and float payloads | Integer and float tiles |

Unknown TIFF compression identifiers are reported as unsupported.

The DNG layer maps both legacy TIFF JPEG XL compression `50002` and DNG 1.7
compression `52546` to the same JPEG XL decoder.

## Current codec limitations

- Lossless JPEG-92 does not yet support pre-DNG-1.1 precision handling, restart
  intervals, Huffman selector `0x10`, or multi-component subsampled and
  half-height tile layouts. A single-component JPEG row that contains multiple
  TIFF tile rows is accepted when total sample count is unchanged. Sony's
  vendor codec separately handles its full-resolution one- and four-component
  tile layouts.
- Lossy JPEG decoding accepts only one 8-bit component. Three-component
  `LinearRaw` payloads are outside the single-band `SampleBuffer` contract.
- JPEG XL decoding accepts only one 8-16-bit channel. Multi-channel `LinearRaw`
  payloads are outside the single-band `SampleBuffer` contract.
- Uncompressed, Deflate, and Zstandard decoding accept only one raw sample per
  pixel in the DNG decoder.
- Vendor packed, word, stripped, lossless-Huffman, block-lossless, CRX,
  predictor-8, and stamped layouts belong to their vendor crates.

## Codec availability

Use `EncodeCompression::Zstd { level }` to select a Zstandard encoding level.
Decoding uses `DecodeCompression::Zstd` because the level is stored in the
stream.

Generic codec implementations are independently feature-gated.

| Feature | Codecs | Provenance |
| --- | --- | --- |
| `deflate` | TIFF Deflate | TIFF 6.0 and `flate2` |
| `jpegxl` | Single-channel JPEG XL | DNG 1.7 and `jxl-oxide` |
| `lossless-jpeg` | JPEG-92 SOF3 predictors | ITU-T T.81 Annex H |
| `lossy-jpeg` | Single-component baseline JPEG | TIFF/DNG and `zune-jpeg` |
| `lzw` | TIFF LZW | TIFF 6.0 and `weezl` |
| `packbits` | TIFF PackBits | TIFF 6.0, independently implemented |
| `zstd` | Zstandard integer/float payloads | Zstandard framing and `zstd` |

The `deflate`, `jpegxl`, `lossless-jpeg`, `lossy-jpeg`, `lzw`, `packbits`, and
`zstd` features enable their matching dispatch variants.
The default feature set includes `lzw` and `packbits`, the standard TIFF
compression pair implemented here; disable defaults to omit them.
Format crates distinguish an implemented codec whose feature is disabled from
a documented compression ID with no Fornax implementation.

Fujifilm's behavior and reference licenses are recorded in
[Fujifilm RAF references](../../docs/reference/decoders/fuji-raf.md).
Phase One's behavior and reference licenses are recorded in
[Phase One IIQ references](../../docs/reference/decoders/phase-one-iiq.md).
Panasonic's behavior and reference licenses are recorded in
[Panasonic RW2 references](../../docs/reference/decoders/panasonic-rw2.md).
