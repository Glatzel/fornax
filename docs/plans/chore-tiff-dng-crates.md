# TIFF and DNG Crate Ownership Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> `superpowers:executing-plans` to implement this plan task-by-task.

**Goal:** Move TIFF container mechanics and DNG semantics out of
`fornax-core`/`fornax-decoder` into focused reusable crates without changing
the decoder contract or the conventional 2x2 RGB Bayer boundary.

**Architecture:** `fornax-tiff` owns generic TIFF tags, values, directories,
parsing, constants, and writing primitives. `fornax-dng` owns DNG-only tags,
codecs, raw transforms, process-metadata conversion, and DNG writing.
`fornax-decoder` remains a thin `IDecoder` adapter. `fornax-core` keeps only
shared image wrappers, stage traits, process metadata, and errors.

## Scope

In scope:

- create `fornax-tiff` and `fornax-dng`;
- move the custom TIFF parser, DNG codecs, DNG metadata, and DNG writer;
- use `Directory` for DNG metadata;
- expose generic TIFF tags and typed TIFF constants;
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
- `fornax_tiff::{Value, Tag, Directory, TiffDirectory, TiffWriter}`.
- `fornax_tiff::constants::{TiffTag, CompressionMethod, PhotometricInterpretation}`.
- `fornax_dng::{DngTag, DngImage}`.
- `fornax_dng::{DngCompression, DngWriter, DngWriteError}`.
- `fornax_dng::metadata::directory_to_json_value(&Directory) ->
  serde_json::Value` behind `metadata-json`.
- `fornax_decoder::Decoder<T>` continues implementing
  `IDecoder<&Path, T>` with the same accessor results.

## Task Checklist

- [x] Add crate manifests, workspace entries, READMEs, and plan documents.
- [x] Move generic TIFF parsing and value handling into `fornax-tiff`.
- [x] Move DNG tags, codecs, transforms, process metadata, and writer into
      `fornax-dng`.
- [x] use a deterministic ordered directory map for DNG metadata.
- [x] add deterministic DNG metadata JSON conversion and tests.
- [x] Reduce `fornax-decoder` to file reading and `IDecoder` delegation.
- [x] Remove `tiff` and unnecessary `image` format features from production
      crates.
- [x] Add compression/layout support tables and update architecture docs.
- [x] Run full validation and record results.
- [x] Add typed TIFF tags and constants to `fornax-tiff`.
- [x] Replace `DngMetadata` with `Directory` throughout decoder interfaces.
- [x] Extend `fornax-tiff` writing with a reusable directory builder.
- [x] Update architecture, crate documentation, and agent instructions.
- [x] Rerun workspace validation and update pull request 650.

### Task 2: Add typed TIFF tags and constants

Write API tests in `fornax-tiff` first for:

- `TiffTag` known and unknown values round-tripping through `u16` and `Tag`;
- `CompressionMethod` preserving known and unknown compression identifiers;
- `PhotometricInterpretation` preserving CFA, LinearRaw, and unknown values.

Implement `fornax-tiff/src/constants.rs` and expose its types through the
public `constants` module. `TiffTag` represents standard TIFF fields previously
declared as `Tag` constants or incorrectly grouped in `DngTag`. `DngTag`
retains only DNG-specific fields and remains directly convertible to and from
`Tag`. `CompressionMethod` moves from the DNG decoder to this module.

Run `cargo test -p fornax-tiff` and `cargo test -p fornax-dng --all-features`.

### Task 3: Use Directory for DNG metadata

Write tests first that merge root and raw IFD metadata into a `Directory`,
including at least one generic `TiffTag` and one DNG-specific `DngTag`.
Assert the resulting map is keyed by `Tag` and preserves typed values.

Remove `DngMetadata`, make `IDecoderDng::dng_metadata` return `Directory`, and
update the default, Rawler, and DNC decoders. Keep deterministic metadata JSON
as a free function over `Directory`.

Run `cargo test -p fornax-dng --all-features` and
`cargo test -p fornax-decoder-rawler --all-features`.

### Task 4: Reuse the TIFF writer directory API

Write a `fornax-tiff` round-trip test for `TiffDirectory`: add generic and
dynamic tags, append sample data, finish the directory, and parse the result.

Move the directory entry builder from `fornax-dng::writer` into
`fornax-tiff::writer`. DNG code keeps DNG structural semantics and codecs but
uses the shared TIFF directory builder for entry accumulation and finalization.

Run `cargo test -p fornax-tiff` and
`cargo test -p fornax-dng --all-features`.

### Task 5: Update ownership documentation and agents

Update architecture, format documentation, both crate READMEs, the root
`AGENTS.md`, and this plan so they describe `Directory` as the metadata model,
the split between `TiffTag` and `DngTag`, and the ownership of generic TIFF
constants and writing helpers.

Run `scripts/agent-validation.sh`.

## Validation Results

- `cargo test --workspace --all-features --exclude compare-demosaic`: 140
  passed across 24 suites.
- `cargo test -p fornax-dng --all-features`: 29 passed.
- `cargo test -p fornax-tiff`: 8 passed.
- `cargo +nightly fmt --check`: passed.
- Repository rustdoc command with `--no-deps`: passed.
- Root clippy flag set: no issues.
- `cargo bench -p bench-decoder --no-run`: passed.
- Production dependency trees for `fornax-core`, `fornax-dng`, and
  `fornax-decoder` contain no external `tiff` crate.
- `git diff --check`: passed.
- `compare-demosaic` rustdoc, clippy, and all 150 package tests passed.

## Files Expected to Change

- `Cargo.toml`, `Cargo.lock`
- `crates/fornax-tiff/**`
- `crates/fornax-dng/**`
- `crates/fornax-core/{Cargo.toml,src/**}`
- `crates/fornax-decoder/{Cargo.toml,src/**}`
- `crates/fornax-decoder-rawler/src/lib.rs`
- `crates/fornax-decoder-dnc/src/wrapper.rs`
- `crates/fornax-preprocessor/src/lib.rs`
- `scripts/agent-validation.sh`, `scripts/agent-validation-test.sh`
- `docs/architecture.md`, `docs/decoder/architecture.md`,
  `docs/decoder/formats/dng.md`
- `docs/references/dng-compression.md`

## Validation

```bash
cargo test --workspace --all-features --exclude compare-demosaic
cargo +nightly fmt --check
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --exclude compare-demosaic --no-deps
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
- Metadata is a `fornax_tiff::Directory` (`BTreeMap<Tag, Value>`); typed
  `TiffTag` and `DngTag` values convert into the dynamic key.
- `TiffDirectory` owns shared writer entry accumulation; DNG-specific
  structural policy remains in `fornax-dng`.
- Unknown compression IDs remain represented and produce typed decode errors.
- BigTIFF input remains supported; automatic BigTIFF output remains deferred.
