# fornax-devtool

`[TARGET]` This is a draft README for the proposed neutral development helper
crate.

## Purpose

`[TARGET]` Provide deterministic fixture paths, synthetic neutral buffers, and
benchmark parameters shared by base tests and benchmarks.

## Scope

`[TARGET]` Generic path setup, synthetic raster data, image export helpers, and
benchmark size/sample/thread controls.

## Must not contain

`[TARGET]` `RawFormat`, camera fixtures, Bayer mosaics, DNG-specific benchmark
inputs, or raw pipeline assumptions.

## Allowed dependencies

`[TARGET]` `fornax-core` and neutral development libraries.

## Features

`[TARGET]` No raw feature. The crate is `publish = false`.

## Related crates

`[TARGET]` Raw-specific helpers belong in `fornax-raw-devtool`.
