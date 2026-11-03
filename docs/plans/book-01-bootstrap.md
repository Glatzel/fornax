# Plan: book-01 — bootstrap the human-facing book

Branch `task/book-01-bootstrap`, worktree `.worktrees/book-01-bootstrap`.
PR title example: `docs: add book skeleton and asset check`. Put `BOOK-01` in the PR body and remove the `BOOK-01` item from `TODO.md` in the same PR (per root `AGENTS.md`).
Do this plan first; `book-02-figures` and `book-03-build` depend on it and must run after it merges, one at a time (they both touch `pixi.toml` and `pixi.lock`).

## Goal

Create `book/`, a human-first, academic-style learning book about the raw pipeline as Fornax implements it (sensor and CFA, preprocess, demosaic, post-process). `docs/` stays agent-only and concise; `book/` is for people.

Readers: (1) a photographer with no image-processing background, (2) a student or engineer who wants equations, derivations and references, (3) the maintainer, who learns the subject by rewriting and reviewing chapters. Demosaic is the centerpiece.

This task builds the skeleton, rules and a CI asset check. It writes no chapters.

## Context a fresh reader needs

- Root `AGENTS.md` rules still apply: stages stay separate; the active demosaic target is conventional 2x2 RGB Bayer, and the book must not claim X-Trans, four-color, monochrome or other CFA support; the `AGENTS.md` closest to changed files wins; do not add items to `TODO.md`; docs-only validation is `git diff --check` plus the repository-configured Markdown lint (`.markdownlint-cli2.jsonc`).
- Read `docs/architecture.md` and `docs/references/` for crate roles and source notes. The book links to them and does not duplicate them.
- Maintainer decisions already made: the human content lives in top-level `book/`; the agent drafts and the maintainer rewrites; a chapter is written only after the code it describes is implemented and merged; figures are generated procedurally (see `book-02-figures`); a PDF build exists for sharing (see `book-03-build`); book tools are declared in `pixi.toml`, never installed globally.

## Scope

In:

- `book/` skeleton, `book/AGENTS.md`, chapter template, notation, glossary, bibliography
- `book/scripts/check_assets.py` and its CI step
- two lines in root `AGENTS.md`

Out:

- chapter prose, figures, build tooling, `pixi.toml` changes (later plans)
- Rust code changes; edits to `docs/architecture.md` or `docs/references/` other than adding links
- committed binary files of any kind

## Layout to create

```text
book/
  AGENTS.md        rules for agents working in book/ (content below)
  README.md        what the book is, reading paths, chapter status table, one Mermaid stage diagram
  template.md      chapter template
  src/
    SUMMARY.md     ordered chapter list; single source of order (GitHub navigation, PDF build, mdBook-compatible)
    notation.md    symbols defined once; includes one rendered formula as a math rendering check
    glossary.md
    bibliography.md
```

No empty chapter stubs. A chapter file appears only when its readiness rule is met.

## Content for `book/AGENTS.md`

### Hard rule: no binary files

Nothing binary is committed: no PNG, JPG, GIF, WebP, TIFF, PDF, raw files, fonts, archives or video. The repo must stay small. This overrides every other rule.

- Allowed in Git: Markdown, Mermaid blocks, small text files (scripts, CSV/JSON).
- Generated SVG is allowed on disk but must stay ignored by Git; it has no size limit.
- SVG is pure vector text. No embedded raster: no `<image>` element, no `data:image/...`, no base64.
- Show pixel behaviour with synthetic SVG (drawn grids, plots, spectra, arrows).
- Real photos and raw comparisons: link to externally hosted images, or generate locally into the ignored `temp/book/`. Never commit the output.
- Every chapter must read correctly with no raster image at all.
- If a binary seems unavoidable, stop and ask.

### Working agreement: agent drafts, maintainer rewrites

1. **Readiness (gate).** A chapter may start only when the code it describes is implemented and merged (example: VNG is merged, so the VNG chapter may be written). Planned algorithms get a "planned" row in the README status table and no prose. Theory chapters (overview, sensor and CFA, the demosaic problem) start once one implementation of that stage exists.
2. **Brief (agent).** Prerequisites, 5 to 10 ranked sources, claims to verify, 5 guiding questions. Short.
3. **Draft (agent).** Follows `template.md`. Every sentence carries a citation or a `file:line`; unverified material is marked `TODO(verify)`. Each section starts with `<!-- DRAFT -->`. Numbers about Fornax come from running it or reading `compare-demosaic` output, never from memory.
4. **Rewrite (maintainer).** Reads the cited sources, rewrites each section in their own words, answers the guiding questions, removes the markers. This step is the point of the project and is never skipped.
5. **Review (agent).** Checks the rewrite against sources and code. Flags errors with the reason and a citation; never silently rewrites substance. Fixes lint, links, formatting; refines figures.
6. **Teach-back (maintainer).** Answers the chapter's "Check yourself" questions in the PR description without notes; the agent grades them against the sources.
7. Only the maintainer marks a chapter done. Any remaining `<!-- DRAFT -->` marker means not done.

### Conventions

- Math: GitHub `$inline$` and `$$display$$`. Define each symbol once in `src/notation.md`.
- Figures: Mermaid for flow, dependency and decision diagrams only. Everything spatial or numeric is an SVG produced by a generator script (`book-02-figures`). Transparent background, mid-contrast colors so light and dark themes both work, mandatory alt text.
- Originality: redraw figures; never copy figures from papers. Link to permissively licensed external samples and record each licence (raw.pixls.us samples are, as far as I know, CC0; verify).
- Citations: keys like `[Malvar2004]` link to `src/bibliography.md` (DOI or stable URL; access date for websites; code as pinned commit permalinks, or the pinned sources under `external/`). Link to `docs/references/<topic>.md` for detail.
- Honesty: every technical claim traces to a cited source or `file:line`. Never invent citations; check that a DOI or URL resolves before using it.
- Theory vs Fornax: always separate what the literature does from what Fornax does. Do not describe stages Fornax lacks as if it had them.

### Chapter template (`book/template.md`)

1. Why it matters: photographer view, plain words, one figure, no equations
2. The idea in one figure
3. The math: assumptions stated, derivations short and complete
4. In Fornax: crate and function, status (implemented / partial / not supported), link to the matching comparison result
5. Failure modes: synthetic SVG figures or links to external images
6. Try it: a small exercise runnable with the example binary or existing tests
7. Go deeper: papers, code, websites and wikis, standards
8. Check yourself: 3 to 5 questions, answers inside `<details>`

## Asset check (`book/scripts/check_assets.py`)

Minimal, Python stdlib only. First read `scripts/` (it has `setup.sh` and `setup.ps1` at least) and the repo's ruff setup (`ruff.toml`) and match their conventions. Behavior:

- Fail if any file added or changed versus the base ref is binary: `git diff --numstat <base>...HEAD`, lines starting `-<TAB>-<TAB>`. Diff-based, so existing binaries elsewhere do not fail CI.
- Base ref is an argument, default `origin/main`.

CI step: read `.github/workflows/` (a `ci.yml` exists) and add a minimal step or small workflow in the existing style, on pull requests, with enough checkout depth for the base diff. The maintainer approved this one `.github/` change for this check only. Do not run the script in later agent sessions; CI owns it. Run `python book/scripts/check_assets.py` once here to prove it works.

## Task checklist

- [ ] Read root `AGENTS.md`, `docs/architecture.md`, `docs/references/`, crate READMEs, `crates/compare-demosaic/`, `scripts/`, `.github/workflows/`, `.markdownlint-cli2.jsonc`, `ruff.toml`; note the patterns to match
- [ ] Create the `book/` files above; README includes the chapter status table (planned / ready / draft / rewrite / done) and one Mermaid diagram of the separate stages (decode, preprocess, demosaic, post-process)
- [ ] Write `book/AGENTS.md` from the section above, hard rule first
- [ ] Create `book/scripts/check_assets.py` and the CI step; run the script once
- [ ] Add `book/build/` and `*.svg` to `.gitignore`; generated figures and build output are never committed
- [ ] Edit root `AGENTS.md`: one Boundaries bullet (no binary files, enforced by CI through `book/scripts/check_assets.py`) and one "Where to look next" pointer to `book/` (human-facing; agents edit it only when asked, following `book/AGENTS.md`). Show the diff.
- [ ] Remove the `BOOK-01` item from `TODO.md`; do not add items (the chapter items are proposed in the maintainer's own TODO update)

## Files expected to change

- new: `book/AGENTS.md`, `book/README.md`, `book/template.md`, `book/src/{SUMMARY,notation,glossary,bibliography}.md`, `book/scripts/check_assets.py`, `docs/plans/book-01-bootstrap.md`
- edit: root `AGENTS.md`, `.gitignore`, one workflow under `.github/workflows/`, `TODO.md` (remove `BOOK-01` only)

## Validation

- `git diff --check` and the repository-configured Markdown lint on changed Markdown
- Ruff on `book/scripts/check_assets.py`, run the way the repo runs it
- `python book/scripts/check_assets.py` once (record output); CI runs it from then on
- No Rust changes, so the cargo checks do not apply; say so in the PR
- Maintainer opens the PR preview on GitHub to confirm math, Mermaid and the README render, light and dark (the agent cannot verify rendering)

## Decisions and notes

Resolved: location `book/`; draft-then-rewrite; readiness follows code; generated SVG stays ignored with no size limit; `temp/book/` is the temporary work directory; the asset check lives at `book/scripts/check_assets.py`; enforcement by CI script; root `AGENTS.md` pointer approved.

Open: none.

Not read when this plan was written: `docs/architecture.md`, `docs/references/`, `scripts/`, `.github/workflows/`, `pixi.toml`, `TODO.md` (GitHub blocked directory and some file access). Follow the repo, not this plan, where they differ.
