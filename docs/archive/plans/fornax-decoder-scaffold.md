# Initialize fornax-decoder Implementation Plan

This archived plan documents the completed Initialize fornax-decoder work.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the planning documents, DNG-shaped scaffold, corpus/benchmark
designs, and backlog entries needed before any native RAW decoder code is
written.

**Architecture:** Introduce a pure-Rust `fornax-decoder` crate whose native API
returns a general CFA-preserving `DecodedRaw` model. Keep the existing
`IDecoder<Path, T>` boundary for conventional 2x2 Bayer input and reject every
other CFA layout with a typed handoff error.

**Tech Stack:** Rust 2024, `fornax-raw-core`, `num_enum`, `strum`, `thiserror`, and
`tiff`.

**Spec:** This task's user request and
[the decoder architecture](../../architecture/decoder.md).

## Global Constraints

- Planning and scaffolding only; do not implement decoder or DNG-writing logic.
- The `fornax-decoder` dependency tree must be pure Rust and contain no C,
  C++, bindings, or `-sys` crate.
- Keep decoding, preprocessing, and demosaicing as separate stages.
- Only a conventional 2x2 RGB Bayer image may cross the preprocessor handoff.
- Do not commit RAW samples, images, DNG files, or other binary files.
- Do not remove LibRaw, `fornax-decoder-dnc`, or `fornax-decoder-rawler`.
- Do not add support for a format or camera outside the approved plans.
- The user explicitly requested `TODO.md` additions; add lines without
  reorganising or removing existing items.

## Review Focus

- A general CFA must remain expressible through decode and DNG export; collapsing
  it into `fornax_raw_core::BayerPattern` during decode is a defect.
- Non-2x2 CFA rejection belongs at the preprocessor handoff and must carry a
  dedicated, matchable error.
- DNG metadata must retain the tag's value type rather than converting every
  field to a string or float.
- Unimplemented construction, protocol, and DNG-writing paths are explicit
  `todo!()` placeholders in this scaffold. Public doctests that exercise them
  use `no_run` until implementation exists.
- Rawler/LibRaw/darktable references must not leak into production dependencies
  or be copied without a licence review.

## Task Checklist

- [x] Read the root/project guidance, existing decoder protocol, Rawler,
      DNC, LibRaw adapter, preprocessor input, `compare-demosaic`, and sample
      downloader.
- [x] Verify the current DNG specification version from Adobe's published
      page.
- [x] Inspect the existing `external/dnglab` and `external/libraw` pinned
      references and read the LGPL-2.1, CDDL-1.0, and GPL-3.0 licences relevant
      to referenced implementations.
- [x] Write the architecture, per-format tier plans, corpus design, and
      benchmark plan.
- [x] Add the compiling `fornax-decoder` public model and errors with explicit
      `todo!()` placeholders for deferred construction and protocol behavior.
- [x] Replace temporary behavior tests with intentional `todo!()` paths and
      `no_run` public examples while the implementation is deferred.
- [x] Revise the public model around image-buffer-backed `MosaicData`, a
      `BTreeMap<DngTag, DngValue>` metadata map, tag-only CFA, strum-backed format
      names, and decoded-image DNG writer methods.
- [x] Add one backlog entry per planned format/tooling/migration work item.
- [x] Run formatting, tests, clippy, documentation tests, and dependency-tree
      checks.
- [ ] Run Markdown lint when a repository-declared tool is available.
- [x] Inspect the complete diff and commit the branch.
- [x] Push the branch and open one semantic pull request.

## Files Expected to Change

- `TODO.md`
- `Cargo.toml`
- `Cargo.lock`
- `crates/fornax-decoder/Cargo.toml`
- `crates/fornax-decoder/README.md`
- `crates/fornax-decoder/src/*.rs`
- `crates/fornax-decoder/tests/scaffold.rs` (removed)
- `docs/architecture/decoder.md`
- `docs/decoder/corpus.md`
- `docs/decoder/benchmark.md`
- `docs/decoder/formats/*.md`
- `docs/_archive/plans/fornax-decoder-scaffold.md`

## Validation

Run from the repository root:

```bash
cargo test -p fornax-decoder
cargo test -p fornax-decoder --doc
cargo check -p fornax-decoder --locked
cargo +nightly fmt -p fornax-decoder --check
cargo +nightly clippy -p fornax-decoder --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p fornax-decoder --no-deps
cargo tree -p fornax-decoder
git -c core.whitespace=cr-at-eol diff --check
```

Run the repository-wide Rust checks from the configured Pixi environment:

```bash
source scripts/setup.sh
cargo test --workspace --all-features
cargo +nightly clippy --workspace --all-features --all-targets -- \
  <exact lint flags in AGENTS.md>
```

Run Markdown lint through the repository-configured tool if available:

```bash
markdownlint-cli2 docs/architecture/decoder.md docs/decoder/corpus.md docs/decoder/benchmark.md docs/decoder/formats/*.md docs/_archive/plans/fornax-decoder-scaffold.md
```

Observed on this branch:

- `cargo test -p fornax-decoder`: 16 passed.
- `cargo test --workspace --all-features`: 269 passed.
- `cargo +nightly fmt --check`: passed.
- `cargo +nightly clippy -p fornax-decoder --all-targets -- -D warnings`:
  passed.
- The exact workspace clippy gate in `AGENTS.md` reports pre-existing failures
  in `fornax-decoder-rawler`, `fornax-raw-pipeline` benchmarks, and
  `fornax-raw-pipeline` benchmark support; none is changed by this task.
  `fornax-decoder` passes clippy separately.
- `markdownlint-cli2` is not installed or declared by this repository, so the
  Markdown lint was not run. This is recorded as a skipped check, not a pass.
- `git diff --check` needs `core.whitespace=cr-at-eol` because the repository
  stores `TODO.md` with CRLF endings; with that setting there are no whitespace
  errors.

The pull request is
[#631](https://github.com/Glatzel/fornax/pull/631).

## Decisions and Notes

- The revised scaffold keeps metadata in an ordinary
  `BTreeMap<DngTag, DngValue>` rather than a `DngMetadata` wrapper. `DngValue`
  re-exports tiff's decoded value type; `DngTag` adds stable names, unknown-tag
  preservation, num_enum primitive conversions, and tiff tag conversions.
- `DecodedRaw` owns image-buffer-backed `MosaicData` plus the metadata map.
  `DecodedRaw::new` is an intentional `todo!()`; matching `ImageWidth` and
  `ImageLength` validation remains required when construction is implemented.
- `IDecoder<Path, T>` integration and the preprocessor Bayer handoff live in a
  private `protocol` module. `DecodedRaw::metadata_by_tag` provides direct
  lookup without operating on the complete map.
- CFA is stored only through its four DNG metadata tags. Typed CFA getters and
  malformed-metadata rejection remain implementation work.
- DNG output is exposed as `DecodedRaw::write_dng(path, options)` and
  `DecodedRaw::write_dng_with(writer, options)`. The tiff encoder will own
  container encoding because it requires `Write + Seek`; no custom writer
  trait is needed. Both methods are intentional `todo!()` placeholders in this
  scaffold.
- `DngVersion::latest()` returns DNG 1.7.1.0 and lossless JPEG-92 is the
  default writer compression.
- The current `Metadata<T>` type cannot represent a CFA larger than 2x2. The
  native DNG-shaped API carries the general CFA, and the current protocol
  remains the Bayer-only handoff. This avoids an unapproved core protocol
  change.
- Adobe DNG Specification 1.7.1.0 (September 2023) is the target. DNG 1.7
  includes BigTIFF.
- rawler/dnglab use LGPL-2.1, LibRaw uses LGPL-2.1 or CDDL-1.0, and darktable
  uses GPL-3.0. They remain references; any copied or adapted implementation
  requires a licence review.
- The corpus acquisition recommendation is to extend
  `scripts/get_raw_samples.py`, not add a binary submodule. This choice and
  the proposed `pass` definition need owner approval before implementation.
- Root guidance says not to add `TODO.md` items, but the explicit user request
  for one line per work item controls. No existing TODO line was changed or
  removed.
