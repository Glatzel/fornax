# fornax-dng

`[TARGET]` This is a draft README for the proposed DNG domain crate.

## Purpose

`[TARGET]` Own DNG vocabulary, DNG-to-process metadata mapping, DNG reading,
writing, and opcodes.

## Scope

`[TARGET]` `DngTag`, DNG-specific compression/photometric identifiers, DNG
reader, DNG writer, DNG opcode application, metadata mapping shared with vendor
decoders, and DNG-specific serialization if retained.

## Must not contain

`[TARGET]` Generic TIFF tag definitions, generic strip/tile layout, codec
implementations, demosaic/color pipeline stages, vendor container decoders, or
facade dispatch.

## Allowed dependencies

`[TARGET]` `fornax-raw-types`, `fornax-tiff`, and `fornax-codec`.

## Features

`[TARGET]` The current metadata serialization feature may remain or be renamed;
that belongs to OPEN 5.

## Related crates

`[TARGET]` Vendor decoders may map their metadata into this crate's DNG-style
language. The raw facade calls the DNG reader as one decoder.
