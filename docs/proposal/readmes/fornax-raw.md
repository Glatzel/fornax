# fornax-raw

`[TARGET]` This is a draft README for the proposed raw facade crate.

## Purpose

`[TARGET]` Compose the raw workspace into a feature-free facade for decoding,
format detection, pipeline processing, and reexports.

## Scope

`[TARGET]` `Decoder`, format detection, raw metadata access, decoder dispatch,
crops, process metadata, and selected reexports from raw domain and pipeline
crates.

## Must not contain

`[TARGET]` Vendor parsing implementations, codec algorithms, TIFF/DNG storage
internals, pipeline algorithm implementations, or Cargo feature switches.

## Allowed dependencies

`[TARGET]` `fornax-raw-types`, `fornax-dng`, `fornax-raw-formats`, and
`fornax-raw-pipeline`.

## Features

`[TARGET]` None. The facade must be feature-free.

## Related crates

`[TARGET]` CLI, Python bindings, comparison tools, and the tracker depend on
this crate rather than directly composing individual decoders and stages.
