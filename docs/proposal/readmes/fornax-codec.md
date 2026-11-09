# fornax-codec

`[TARGET]` This is a draft README for the proposed neutral codec crate.

## Purpose

`[TARGET]` Decode byte streams into neutral sample buffers and encode supported
buffers into independently stored chunks.

## Scope

`[TARGET]` One tag-identified generic entry point, `decode(DecodeRequest)`;
uncompressed, Deflate, Zstandard, LZMA, LZW, PackBits if retained, lossless
JPEG, lossy JPEG, and JPEG XL; typed vendor codec functions.

## Must not contain

`[TARGET]` Raw workflow types, Bayer meaning, metadata-to-process conversion,
TIFF/DNG directory policy, camera model heuristics, or pipeline stages.

## Allowed dependencies

`[TARGET]` `fornax-core` and codec implementation libraries. No raw crate.

## Features

`[TARGET]` Uncompressed is always on. Other generic codecs have independent
features. A disabled codec returns `UnsupportedCompression` naming the feature.
Vendor feature placement follows OPEN 3.

## Related crates

`[TARGET]` `fornax-tiff` may consume only the generic/raster mapping through
its optional `raster` feature. Raw format crates call generic and typed vendor
decode functions.
