# fornax-py

`[TARGET]` This is a draft README for the proposed Python binding crate.

## Purpose

`[TARGET]` Expose supported raw decoding and pipeline operations to Python.

## Scope

`[TARGET]` A deliberately small binding over `fornax-raw`, with explicit
parameter serialization and array conversion.

## Must not contain

`[TARGET]` Its own decoder policy, alternate LibRaw/DNG Converter pipeline, or
old facade API names.

## Allowed dependencies

`[TARGET]` `fornax-raw` plus PyO3/Python integration crates.

## Features

`[TARGET]` No feature list is mandated. The current crate remains excluded
until buildability and API are resolved.

## Related crates

`[TARGET]` Belongs to the raw tools group; current source references removed
facade APIs and is not a usable baseline.
