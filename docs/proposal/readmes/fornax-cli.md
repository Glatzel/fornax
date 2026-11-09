# fornax-cli

`[TARGET]` This is a draft README for the proposed raw workspace CLI.

## Purpose

`[TARGET]` Convert raw files to regenerated DNG, rendered TIFF/JPEG, and
optional metadata JSON.

## Scope

`[TARGET]` Input discovery, configuration, output planning, decoding through
the facade, pipeline invocation, and output writing.

## Must not contain

`[TARGET]` Decoder implementations, codec implementations, pipeline algorithms,
or TIFF/DNG parsing internals.

## Allowed dependencies

`[TARGET]` `fornax-raw` facade and required base APIs. It should not directly
compose vendor decoders and pipeline stages.

## Features

`[TARGET]` No feature list is mandated by the target.

## Related crates

`[TARGET]` Uses `fornax-raw` and stays in the tools group of the raw workspace.
