# fornax-raw-formats

`[TARGET]` This is a draft README for the proposed vendor-format crate.

## Purpose

`[TARGET]` Implement camera/vendor RAW containers and translate them into raw
types and metadata understood by the facade.

## Scope

`[TARGET]` Sony ARW/NRW now; NEF, CR3, and other formats later. Includes vendor
container parsing, private metadata, format detection helpers, and typed vendor
codec dispatch.

## Must not contain

`[TARGET]` Generic TIFF parsing, generic codec algorithms owned by
`fornax-codec`, DNG parsing/writing policy, pipeline stages, or the top-level
decoder facade.

## Allowed dependencies

`[TARGET]` `fornax-raw-types`, `fornax-dng`, `fornax-tiff`, and
`fornax-codec`. It must not depend on `fornax-raw-pipeline`.

## Features

`[TARGET]` The target does not mandate vendor-selection features. Compile-cost
and feature strategy for future formats is an evidence gap.

## Related crates

`[TARGET]` `fornax-raw` owns format dispatch; vendor codecs may move to
`fornax-codec` under OPEN 3.
