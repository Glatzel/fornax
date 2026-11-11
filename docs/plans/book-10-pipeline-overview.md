# Plan: book-10 - pipeline overview

Branch `task/book-10-pipeline-overview`, worktree
`.worktrees/book-10-pipeline-overview`.
PR title: `docs: add L00 pipeline overview`.
Put `BOOK-10` in the PR body and remove the `BOOK-10` item from `TODO.md` in
the same pull request.

## Goal

Draft the L00 chapter for photographers and engineering readers. It explains
how one RAW input becomes one RGB output through Fornax's separate decode,
preprocess, demosaic, and RGB-output stages, and maps each stage boundary to
the crate that owns it.

## Scope

In:

- add the L00 draft chapter, following `book/template.md` and keeping every
  technical sentence cited;
- register the chapter in `book/src/SUMMARY.md` and change its status in
  `book/README.md` from ready to draft;
- add the Adobe DNG 1.7.1.0 source used for raw-image terminology to the book
  bibliography;
- remove only `BOOK-10` from `TODO.md`.

Out:

- Rust or crate API changes;
- algorithm explanations that belong in L01 and later chapters;
- new figures, scripts, generated SVG, or binary assets;
- claims of support for non-2x2 CFA layouts.

## Source brief

Prerequisites: decode, preprocess, and demosaic are implemented and merged;
the book skeleton and procedural figure support are already on `main`.

Ranked sources:

1. Adobe, Digital Negative Specification 1.7.1.0: raw-image terminology and
   the metadata concepts represented by Fornax. URL checked with HTTP 200 on
   2026-11-04.
2. `docs/architecture.md`, lines 40-102: authoritative stage boundaries, crate
   responsibilities, supported Bayer scope, and example location.
3. `crates/fornax-core/src/protocol.rs`, lines 6-40: decoder, preprocessor, and
   postprocessor trait inputs and outputs.
4. `crates/fornax-decoder-rawler/src/lib.rs`, lines 14-44 and 61-110: active
   mosaic construction, geometry, and CFA acceptance.
5. `crates/fornax-core/src/metadata.rs`, lines 5-92, and
   `crates/fornax-core/src/bayer.rs`, lines 23-94: metadata fields, level
   semantics, and supported 2x2 Bayer phases.
6. `crates/fornax-preprocessor/src/lib.rs`, lines 11-80: black/white-level
   normalization and clipping.
7. `crates/fornax-postprocessor/src/protocol.rs`, lines 20-89, and
   `crates/fornax-postprocessor/src/lib.rs`, lines 23-63: demosaic output and
   RGB-domain white-balance behavior.
8. `crates/fornax/examples/pipeline.rs`, lines 12-60, and
   `crates/compare-demosaic/README.md`, lines 128-142: runnable end-to-end
   example and evaluation evidence.

Claims to verify:

- `decode_active` returns an active-area mosaic plus common metadata.
- The Rawler adapter accepts CFA payloads only when they are 2x2 RGB Bayer.
- Preprocessing subtracts black level, divides by the white-level range, and
  clamps negatives to zero.
- `PostProcessor` composes demosaicing and optional white balance, while
  `default_crop` is not applied automatically.
- The example emits stage-named TIFF files from the same input.

Guiding questions:

1. What information exists after decode that did not exist as RGB?
2. Which stage changes sample meaning rather than only sample geometry?
3. Why are demosaic and RGB output shown separately even when one Rust type
   composes them?
4. Which incorrect assumptions fail early, and which survive into output?
5. What should a reader inspect to connect a crate to a pipeline arrow?

## Task checklist

- [x] Add `book/src/l00-pipeline-overview.md` with all eight template sections,
  draft markers, a Mermaid flow figure, and cited file-line references.
- [x] Replace the planned L00 entry in `book/src/SUMMARY.md` with a chapter
  link and change the L00 status in `book/README.md` to draft.
- [x] Add the Adobe DNG standard to `book/src/bibliography.md`.
- [x] Remove `[BOOK-10]` from `TODO.md`.
- [x] Run `git diff --check`, Markdown lint on changed Markdown, the book asset
  check, and `cargo check -p fornax --example pipeline`.
- [x] Review the diff and record validation outcomes in this plan and the PR.

## Files expected to change

- new: `book/src/l00-pipeline-overview.md`,
  `docs/plans/book-10-pipeline-overview.md`
- edit: `book/README.md`, `book/src/SUMMARY.md`,
  `book/src/bibliography.md`, `TODO.md`

## Validation

- `git diff --check`
- `pixi exec -s markdownlint-cli2 markdownlint-cli2 <changed Markdown>`
- `python3 book/scripts/check_assets.py origin/main`
- `pixi run -e dev cargo check -p fornax --example pipeline`, with
  `LIBRAW_ROOT` and `LD_LIBRARY_PATH` set to `.pixi/envs/dev` and
  `.pixi/envs/dev/lib` as required by the repository
- `pixi run -e dev cargo run -p fornax --example pipeline -- <fixture CR2>`
- no full Rust suite is required because this changes documentation only

## Decisions and notes

- The L00 figure is Mermaid because it explains stage flow rather than spatial
  or numeric behavior.
- The chapter labels conventional 2x2 RGB Bayer as implemented and all other
  CFA layouts as unsupported.
- The chapter describes white balance as RGB-domain output work composed by
  `PostProcessor`; it does not redraw that implementation as a separate Rust
  pipeline type.
- The existing normalization symbols in `book/src/notation.md` are sufficient,
  so no notation changes are planned.

## Progress log

- 2026-11-04: Drafted L00, registered it in the summary, marked it as draft,
  removed it from `TODO.md`, and added Adobe DNG 1.7.1.0 to the bibliography.
- 2026-11-04: `git diff --check`, Markdown lint, and the book asset check
  passed.
- 2026-11-04: The example compile check passed with the repository's LibRaw
  environment. Running the example against
  `external/raw-images/images/colorchart-eos-7d.cr2` produced the decoded,
  preprocessed, and demosaiced TIFFs for both LibRaw and Rawler under
  `temp/pipeline/`.
