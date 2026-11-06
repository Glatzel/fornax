# TIFF and DNG Crate Ownership Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> `superpowers:executing-plans` to implement this plan task-by-task.

**Goal:** Move TIFF container mechanics and DNG semantics out of
`fornax-core`/`fornax-decoder` into focused reusable crates without changing
the decoder contract or the conventional 2x2 RGB Bayer boundary.

**Architecture:** `fornax-tiff` owns generic TIFF parsing/value/writing
primitives. `fornax-dng` owns DNG tags, metadata, compression selection,
decoding codecs, raw transforms, process-metadata conversion, and DNG writing.
`fornax-decoder` remains a thin `IDecoder` adapter. `fornax-core` keeps only
shared pipeline types, traits, process metadata, and errors.

## Scope

In scope:

- create `fornax-tiff` and `fornax-dng`;
- move the custom TIFF parser, DNG codecs, DNG metadata, and DNG writer;
- introduce `DngMetadata` with cheap `Arc` clones;
- replace compression constants with `CompressionMethod`;
- add deterministic DNG metadata JSON support;
- add format support tables and architecture updates;
- remove direct `tiff` dependencies from production DNG parsing/writing.

Out of scope:

- decode hot-path optimization;
- lazy preprocessing;
- Zstandard;
- CLI implementation;
- auxiliary TIFF directory preservation or byte-identical DNG export.

## Public Interfaces

- `fornax_tiff::parse(&[u8]) -> Result<ParsedTiff, TiffError>`.
- `fornax_tiff::{Value, Tag, Directory, TiffWriter}`.
- `fornax_dng::{DngTag, DngMetadata, DngImage, CompressionMethod}`.
- `fornax_dng::{DngCompression, DngWriter, DngWriteError}`.
- `DngMetadata::to_json_value() -> serde_json::Value` behind
  `metadata-json`.
- `fornax_decoder::Decoder<T>` continues implementing
  `IDecoder<&Path, T>` with the same accessor results.

## Task Checklist

- [x] Add crate manifests, workspace entries, READMEs, and plan documents.
- [x] Move generic TIFF parsing and value handling into `fornax-tiff`.
- [x] Move DNG tags, codecs, transforms, process metadata, and writer into
      `fornax-dng`.
- [x] Make `DngMetadata` an `Arc`-backed cheap-to-clone map.
- [x] Replace decoder compression constants with `CompressionMethod`.
- [x] Add deterministic DNG metadata JSON conversion and tests.
- [x] Reduce `fornax-decoder` to file reading and `IDecoder` delegation.
- [x] Remove `tiff` and unnecessary `image` format features from production
      crates.
- [x] Add compression/layout support tables and update architecture docs.
- [x] Run full validation and record results.

## Validation Results

- `cargo test --workspace --all-features --exclude compare-demosaic`: 131
  passed across 21 suites.
- `cargo test -p fornax-dng --all-features`: 25 passed.
- `cargo +nightly fmt --check`: passed.
- Repository rustdoc command: passed.
- Root clippy flag set: no issues.
- `cargo bench -p bench-decoder --no-run`: passed.
- Production dependency trees for `fornax-core`, `fornax-dng`, and
  `fornax-decoder` contain no external `tiff` crate.
- `git diff --check`: passed.
- `compare-demosaic` package validation could not run because its checkout
  lacks the setup-generated `darktable amaze.cc` reference source. The changed
  dependency only enables the package's existing `image` PNG feature.

## Files Expected to Change

- `Cargo.toml`, `Cargo.lock`
- `crates/fornax-tiff/**`
- `crates/fornax-dng/**`
- `crates/fornax-core/{Cargo.toml,src/**}`
- `crates/fornax-decoder/{Cargo.toml,src/**}`
- `crates/fornax-decoder-rawler/src/lib.rs`
- `crates/fornax-decoder-dnc/src/wrapper.rs`
- `docs/architecture.md`, `docs/decoder/architecture.md`,
  `docs/decoder/formats/dng.md`
- `docs/references/dng-compression.md`

## Validation

```bash
cargo test --workspace --all-features --exclude compare-demosaic
cargo +nightly fmt --check
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --exclude compare-demosaic
cargo clippy --workspace --all-features --all-targets --exclude compare-demosaic -- <root AGENTS.md flags>
cargo bench -p bench-decoder --no-run
git diff --check
```

Additional checks:

- `cargo tree -p fornax-core` has no direct `tiff` dependency.
- `cargo tree -p fornax-dng` has no `tiff` dependency.
- `cargo tree -p fornax-decoder` has no direct `tiff` dependency.
- Metadata JSON snapshots cover every scalar/list/ASCII/rational/byte family.

## Decisions and Notes

- `tiff` may remain in the workspace through `image` encoders used by the CLI
  or devtools, but it is not part of Fornax's DNG parser or writer.
- `DngMetadata` is immutable by default and clones its `Arc`; structural
  construction uses explicit mutation methods.
- Unknown compression IDs remain represented and produce typed decode errors.
- BigTIFF input remains supported; automatic BigTIFF output remains deferred.
