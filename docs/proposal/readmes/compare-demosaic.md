# compare-demosaic

`[TARGET]` This is a draft README for the proposed comparison tool.

## Purpose

`[TARGET]` Compare Fornax demosaic output with pinned LibRaw, darktable, and
RawTherapee references at the same active-Bayer-to-RGB boundary.

## Scope

`[TARGET]` Case definitions, reference adapters, canonical JSON reports,
baseline diffs, waivers, generated README tables, and optional artifacts.

## Must not contain

`[TARGET]` Production decoder dependencies on reference libraries or changes to
the production color pipeline for comparison convenience.

## Allowed dependencies

`[TARGET]` Raw facade/pipeline APIs plus development-only C/C++ references.
Keep the crate excluded from default members.

## Features

`[TARGET]` No feature list is mandated.

## Related crates

`[TARGET]` Stays a raw-workspace tool; generated tables must be refreshed with
its own existing CLI.
