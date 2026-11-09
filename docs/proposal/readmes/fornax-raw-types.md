# fornax-raw-types

`[TARGET]` This is a draft README for the proposed raw domain crate.

## Purpose

`[TARGET]` Define the shared raw-domain language used by decoders, pipeline
stages, the facade, and tools.

## Scope

`[TARGET]` `BayerPattern`, `Levels`, `ProcessMetadata`, `CroppedMosaic`,
`NormalizedMosaic`, `RawFormat`, decoder/pre/post protocols, and color-matrix
helpers.

## Must not contain

`[TARGET]` Decoder implementations, TIFF/DNG parsing or writing, codec
algorithms, pipeline implementations, vendor metadata heuristics, or
external-format detection magic.

## Allowed dependencies

`[TARGET]` `fornax-core` and light numeric/image support. No `fornax-dng`,
`fornax-raw-formats`, or `fornax-raw-pipeline`.

## Features

`[TARGET]` No feature is mandated. Final naming is OPEN 5.

## Related crates

`[TARGET]` `fornax-dng`, `fornax-raw-formats`, `fornax-raw-pipeline`, and
`fornax-raw` depend on this crate.
