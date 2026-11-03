# Initialize fornax-decoder Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the planning documents, DNG-shaped scaffold, corpus/benchmark
designs, and backlog entries needed before any native RAW decoder code is
written.

**Architecture:** Introduce a pure-Rust `fornax-decoder` crate whose native API
returns a general CFA-preserving `DecodedRaw` model. Keep the existing
`IDecoder<Path, T>` boundary for conventional 2x2 Bayer input and reject every
other CFA layout with a typed handoff error.

**Tech Stack:** Rust 2024, `fornax-core`, and `thiserror`.

**Spec:** This task's user request and
[the decoder architecture](../decoder/architecture.md).

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
  it into `fornax_core::BayerPattern` during decode is a defect.
- Non-2x2 CFA rejection belongs at the preprocessor handoff and must carry a
  dedicated, matchable error.
- DNG metadata must retain the tag's value type rather than converting every
  field to a string or float.
- The scaffold's public stubs must fail as explicit unsupported operations,
  never with `todo!()`, panic, or a misleading corrupt-input result.
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
- [x] Add the compiling `fornax-decoder` public model, errors, CFA handoff
  check, and unsupported stubs.
- [x] Add tests for unsupported stubs, typed metadata storage, and CFA
  handoff behavior.
- [x] Add one backlog entry per planned format/tooling/migration work item.
- [x] Run formatting, tests, clippy, documentation tests, and dependency-tree
  checks.
- [ ] Run Markdown lint when a repository-declared tool is available.
- [ ] Inspect the complete diff and commit the branch.
- [ ] Push the branch and open one semantic pull request.

## Files Expected to Change

- `TODO.md`
- `Cargo.lock`
- `crates/fornax-decoder/Cargo.toml`
- `crates/fornax-decoder/README.md`
- `crates/fornax-decoder/src/*.rs`
- `crates/fornax-decoder/tests/scaffold.rs`
- `docs/decoder/architecture.md`
- `docs/decoder/corpus.md`
- `docs/decoder/benchmark.md`
- `docs/decoder/formats/*.md`
- `docs/plans/chore-fornax-decoder-scaffold.md`

## Validation

Run from the repository root:

```bash
cargo test -p fornax-decoder --test scaffold
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
cargo +nightly clippy --workspace --all-features --all-targets -- -D warnings
```

Run Markdown lint through the repository-configured tool if available:

```bash
markdownlint-cli2 docs/decoder/architecture.md docs/decoder/corpus.md docs/decoder/benchmark.md docs/decoder/formats/*.md docs/plans/chore-fornax-decoder-scaffold.md
```

Observed on this branch:

- `cargo test -p fornax-decoder`: 33 passed.
- `cargo test --workspace --all-features`: 213 passed.
- `cargo +nightly fmt --check`: passed.
- `cargo +nightly clippy -p fornax-decoder --all-targets -- -D warnings`:
  passed.
- Full workspace clippy stops on pre-existing
  `clippy::too_many_arguments` failures in
  `crates/fornax-postprocessor/src/demosaic/ahd.rs` and
  `crates/libraw/src/c_api/data_loading.rs`; neither file is changed by this
  task.
- `markdownlint-cli2` is not installed or declared by this repository, so the
  Markdown lint was not run. This is recorded as a skipped check, not a pass.
- `git diff --check` needs `core.whitespace=cr-at-eol` because the repository
  stores `TODO.md` with CRLF endings; with that setting there are no whitespace
  errors.

## Decisions and Notes

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
