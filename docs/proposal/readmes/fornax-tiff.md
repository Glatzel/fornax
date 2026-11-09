# fornax-tiff

`[TARGET]` This is a draft README for the proposed TIFF-only base crate.

## Purpose

`[TARGET]` Model TIFF data types, tags, values, directories, parsing, and
classic writing without raw-image policy.

## Scope

`[TARGET]` Open tag identifiers, TIFF 6.0 and TIFF/EP constants grouped by
specification, classic TIFF, BigTIFF parsing, classic writer, and optional
strip/tile raster layout.

## Must not contain

`[TARGET]` DNG tag vocabulary, DNG opcodes, camera metadata policy,
`ProcessMetadata`, Bayer layouts, raw decoders, or GeoTIFF constants.

## Allowed dependencies

`[TARGET]` `fornax-core`; `fornax-codec` only when the optional `raster`
feature is enabled.

## Features

`[TARGET]` `raster` adds strip/tile layout and TIFF `Compression` to codec
request mapping.

## Related crates

`[TARGET]` `fornax-dng`, `fornax-raw-formats`, and a future geography
repository may consume this crate.
