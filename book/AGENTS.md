# Agent instructions - Fornax book

This directory is the human-facing book about the RAW pipeline as Fornax
implements it. Repository-wide rules in the root `AGENTS.md` still apply.

## Hard rule: no binary files

Nothing binary is committed: no PNG, JPG, GIF, WebP, TIFF, PDF, RAW files,
fonts, archives, or video. The repository must stay small. This rule overrides
every other rule.

- Git may contain Markdown, Mermaid blocks, and small text files such as
  scripts, CSV, and JSON.
- Generated SVG may exist on disk but must remain ignored by Git. Generated
  vector figures have no size limit.
- SVG must be pure vector text. Do not embed a raster with `<image>`,
  `data:image/...`, or base64 data.
- Show pixel behavior with synthetic SVG: drawn grids, plots, spectra, and
  arrows.
- Link to externally hosted real photos and RAW comparisons, or generate them
  locally under the ignored `temp/book/` directory. Never commit the output.
- Every chapter must remain understandable without any raster image.
- If a binary asset seems unavoidable, stop and ask the maintainer.

CI enforces this gate through `book/scripts/check_assets.py`.

## Working agreement: agent drafts, maintainer rewrites

1. **Readiness gate.** A chapter may start only when the code it describes is
   implemented and merged. Planned algorithms get a planned row in
   `README.md` and no prose. Theory chapters may start once one implementation
   of the stage exists.
2. **Brief.** Record prerequisites, 5 to 10 ranked sources, claims to verify,
   and 5 guiding questions. Keep it short.
3. **Draft.** Follow `template.md`. Every sentence carries a citation or a
   `file:line` reference, and unverified material is marked
   `TODO(verify)`. Start each section with `<!-- DRAFT -->`. Measure numbers
   about Fornax by running it or reading comparison output; never recall them
   from memory.
4. **Rewrite.** The maintainer reads the cited sources, rewrites every section
   in their own words, answers the guiding questions, and removes the draft
   markers. This step is the point of the project and is never skipped.
5. **Review.** An agent checks the rewrite against sources and code. Flag
   errors with the reason and a citation; do not silently rewrite substance.
   Fix lint, links, and formatting, and refine figures.
6. **Teach-back.** The maintainer answers the chapter's "Check yourself"
   questions in the pull request description without notes. The agent grades
   the answers against the sources.
7. **Completion.** Only the maintainer marks a chapter done. Any remaining
   `<!-- DRAFT -->` marker means the chapter is not done.

## Conventions

- Use GitHub math with `$inline$` and `$$display$$`. Define each symbol once in
  `src/notation.md`.
- Use Mermaid only for flow, dependency, and decision diagrams. Generate
  spatial or numeric figures as SVG with a script.
- Give every figure mandatory alt text. Use a transparent background and
  mid-contrast colors that remain legible in light and dark themes.
- Redraw figures; never copy a figure from a paper. Link to external samples
  only after verifying their licence and recording it in
  `src/bibliography.md`.
- Use citation keys such as `[Malvar2004]` that link to entries in
  `src/bibliography.md`. Record a DOI or stable URL, an access date for a
  website, and a pinned commit URL for code. Link to
  `docs/reference/<category>/<topic>.md` for repository research notes.
- Every technical claim must trace to a cited source or a `file:line`
  reference. Never invent a citation; verify that a DOI or URL resolves before
  using it.
- Separate what the literature does from what Fornax does. Do not describe a
  stage Fornax lacks as if Fornax implements it.
- Support conventional 2x2 RGB Bayer input only. Do not claim X-Trans,
  four-color, monochrome, or other CFA layouts.

## Chapter template

New chapters follow `template.md`:

1. Why it matters
2. The idea in one figure
3. The math
4. In Fornax
5. Failure modes
6. Try it
7. Go deeper
8. Check yourself

The "In Fornax" section names the crate and function and labels behavior as
implemented, partial, or not supported. "Try it" must use an existing example
binary, test, or comparison command.

## Generated and temporary files

- `book/build/` and `*.svg` are ignored by Git.
- `temp/book/` is the temporary directory for generated figures, rendered
  previews, externally sourced samples, and other untracked artifacts.
- Do not weaken `.gitignore` or the asset checker to admit a generated binary.
