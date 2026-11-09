# fornax-core

`[TARGET]` This is a draft README for the proposed neutral base crate; it is
not the live package README.

## Purpose

`[TARGET]` Provide neutral sample/image buffers, shared error plumbing, and
byte-order-style basics with no raw-image workflow meaning.

## Scope

`[TARGET]` Include generic raster storage, primitive sample bounds, basic
serialization-neutral byte order, and errors that do not name Bayer, levels,
DNG, cameras, or pipeline stages.

## Must not contain

`[TARGET]` Bayer patterns, black/white levels, raw metadata, decoder protocols,
demosaic protocols, color profiles, TIFF tags, DNG tags, fixtures, or camera
format detection.

## Allowed dependencies

`[TARGET]` No `fornax-*` crate dependencies.

## Features

`[TARGET]` No raw feature. The image buffer shape remains OPEN 1.

## Related crates

`[TARGET]` `fornax-codec`, `fornax-tiff`, and `fornax-devtool` may depend on
this crate. Raw domain types belong in `fornax-raw-types`.
