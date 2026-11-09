# fornax-raw-pipeline

`[TARGET]` This is a draft README for the proposed merged raw pipeline crate.

## Purpose

`[TARGET]` Provide normalization, demosaic, and color processing for raw Bayer
mosaics.

## Scope

`[TARGET]` `normalize`: black level, white level, and normalization only.
`demosaic`: the current Bayer algorithms. `color`: white balance, color
matrices, and output color spaces.

## Must not contain

`[TARGET]` File parsing, codecs, vendor metadata heuristics, decoder dispatch,
TIFF/DNG storage policy, or dependencies on DNG/format crates.

## Allowed dependencies

`[TARGET]` `fornax-raw-types` only among `fornax-*` crates.

## Features

`[TARGET]` No pipeline feature is mandated.

## Related crates

`[TARGET]` Only `fornax-raw` composes this crate with decoders for end-to-end
processing.
