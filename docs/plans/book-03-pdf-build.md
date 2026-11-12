# Plan: book-03 — build a shareable PDF

Branch `task/book-03-build`, worktree `.worktrees/book-03-build`.
PR title example: `docs: add book PDF build`. Put `BOOK-03` in the PR body and remove its `TODO.md` item in the same PR.
Requires `book-01-bootstrap` merged. Run after `book-02-figures` (shared `pixi.toml` and `pixi.lock`), or before it if the maintainer prefers; never in parallel.

## Goal

One command produces a PDF of the book that the maintainer can send to friends who do not use GitHub:

```text
pixi run -e book book-pdf     ->     book/build/fornax-book.pdf
```

The PDF is a build artifact. It is never committed (hard rule in `book/AGENTS.md`). Tools live in `pixi.toml`, not on the machine.

## Context a fresh reader needs

- Read `book/AGENTS.md` first. Content is GitHub-flavored Markdown with `$...$` math, Mermaid blocks and SVG figures; chapter order comes from `book/src/SUMMARY.md`.
- The maintainer's requirements: a PDF for sharing; book tools only through `pixi.toml`; for now only a plan, so this task begins with a spike.
- Planned route: **pandoc with the Typst PDF engine**. pandoc reads the Markdown, converts TeX math to Typst math, and embeds SVG figures; Typst produces the PDF. Both are cross-platform tools; confirm their conda-forge availability on every platform `pixi.toml` declares, win-64 included (conda-forge has `typst-py` for win-64; check for a `typst` CLI package and for `pandoc`).
- Fallbacks, only after reporting spike results and asking: pandoc to LaTeX through `tectonic`; or an mdBook HTML build printed to PDF by a headless browser (heavier).
- mdBook as a website is out of scope. If wanted later it is a separate task; `book/src/SUMMARY.md` is already mdBook-compatible.

## Scope

In:

- `book` pixi feature additions (pandoc, Typst, and whatever the spike proves necessary)
- a build script and pixi task `book-pdf`
- a spike on one throwaway two-chapter sample, results recorded in this plan

Out:

- real chapters, new figures
- committing any PDF or build output
- CI workflows, release attachment, GitHub Pages (propose, do not add)

## Spike (do first, report before the rest)

Build a PDF from a temporary sample under the ignored `temp/book/` directory that contains: a numbered heading structure across two files, inline and display math (use `\dfrac`, sums, subscripts), a table, a GitHub alert block (`> [!WARNING]`), a relative link between two chapter files, a link to `bibliography.md#<key>`, a generated SVG figure, and a Mermaid block. Record for each item whether it works, how it looks, and the fix if not.

Known risks to test, not assume:

1. Relative `.md` links between chapters must become in-document links in the PDF (likely a small pandoc Lua filter).
2. Whether the GFM alert syntax is understood by the installed pandoc version.
3. Whether every TeX construct used in the book converts to Typst math cleanly.
4. **Mermaid.** It renders on GitHub but not in a PDF. Options, choose by evidence:
   - (a) pre-render Mermaid to SVG at build time into `book/build/` using mermaid-cli (Node plus a headless Chromium); only acceptable if it installs through pixi on Windows and Linux without manual steps
   - (b) keep Mermaid for GitHub only and give diagrams that matter a generated-SVG twin from `book-02-figures` tooling
   - (c) restrict Mermaid usage in the book and accept a text fallback in the PDF
   Recommend one in the plan notes with the reasons.
5. Fonts: the PDF must embed fonts and render Greek letters and math without installing system fonts.

## Task checklist

- [ ] Read `pixi.toml` (features, environments, platforms, existing tasks) and `scripts/`; match their conventions
- [ ] Spike as above; write findings into "Decisions and notes" and stop for the maintainer if option (a) is needed or a fallback route is required
- [ ] Add the proven dependencies to the `book` feature; update `pixi.lock`
- [ ] `book/scripts/book_build.py` (stdlib; follow existing `scripts/` conventions): read `book/src/SUMMARY.md`, build the ordered file list, call pandoc with the chosen filters and metadata (title, version from the repo, date), write `book/build/fornax-book.pdf`
- [ ] Pixi task `book-pdf` in the `book` environment
- [ ] Build the PDF from the real `book/src/` content as it exists; it may be nearly empty, and that is fine. Record file size and page count
- [ ] Confirm `book/build/` is ignored and `git status` shows no PDF
- [ ] Remove the `BOOK-03` item from `TODO.md`

## Files expected to change

- new: `book/scripts/book_build.py`, possibly one Lua filter under `book/scripts/`, `docs/plans/book-03-build.md`
- edit: `pixi.toml`, `pixi.lock`, `TODO.md` (remove `BOOK-03` only)

## Validation

- `git diff --check` and the repository-configured Markdown lint on changed Markdown
- Ruff on Python files, run the way the repo runs it
- `pixi run -e book book-pdf` succeeds from a clean worktree; report page count and size
- PDF checked by eye: math renders, SVG figures appear, internal links jump, fonts embedded
- No Rust changes, so the cargo checks do not apply; say so in the PR
- Do not hand-run `book/scripts/check_assets.py`; CI owns it

## Decisions and notes

- Sharing: the maintainer builds the PDF locally and sends it. A later task may attach it to a GitHub Release or build it in CI; both touch `.github/` and need approval.
- Optional later task: mdBook website (`mdbook` is on conda-forge; `mdbook-katex` and `mdbook-mermaid` are usually installed with `cargo install`, conda-forge availability unconfirmed). Not part of this task.
- Not read when this plan was written: `pixi.toml`, `scripts/`, `.github/workflows/`.
