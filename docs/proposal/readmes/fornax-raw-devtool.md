# fornax-raw-devtool

`[TARGET]` This is a draft README for the proposed raw development helper
crate.

## Purpose

`[TARGET]` Provide raw-specific fixture paths, mosaic benchmarks, and synthetic
raw test helpers.

## Scope

`[TARGET]` `RawFormat` fixture lookup, DNG writer benchmark input selection,
`RawMosaic` byte accounting, and raw mosaic/image helpers.

## Must not contain

`[TARGET]` Base-workspace-neutral bench controls or production decoding and
pipeline policy.

## Allowed dependencies

`[TARGET]` `fornax-raw-types`, `fornax-devtool`, and test/benchmark support
crates.

## Features

`[TARGET]` No feature list is mandated; `publish = false`.

## Related crates

`[TARGET]` Complements `fornax-devtool`; raw helpers do not belong in the base
workspace.
