# decoder-tracker

`[TARGET]` This is a draft README for the proposed decoder tracker.

## Purpose

`[TARGET]` Validate the pointer-only raw corpus manifest and run deterministic,
sharded decoder regression checks against a committed CSV baseline.

## Scope

`[TARGET]` Selection, sharding, isolated decoder runs, stage classification,
CSV merge/diff, DuckDB-backed aggregation, and generated README status.

## Must not contain

`[TARGET]` RAW payloads, decoder implementations, or automatic baseline updates
outside the manual reviewed workflow.

## Allowed dependencies

`[TARGET]` `fornax-raw`, `fornax-dng`, `fornax-tiff`, and tracker development
tools.

## Features

`[TARGET]` No feature list is mandated; `publish = false`.

## Related crates

`[TARGET]` Remains a raw-workspace tool and must follow the manual corpus
workflow.
