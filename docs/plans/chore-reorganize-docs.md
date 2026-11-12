# Documentation Reorganization Implementation Plan

This plan records the approved documentation reorganization and its validation steps.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reorganize `docs/` into a purpose-based tree that is navigable by
humans and agents while preserving document history and current content.

**Architecture:** Keep current architecture, decoder, reference, and active
plan documents in small purpose-based folders; move completed plans, design
records, and dated refactor evidence under `docs/_archive/`. Preserve every
existing document with `git mv`, repair all links, and add a complete index and
contribution conventions.

**Tech Stack:** Git, Markdown, repository shell validation, and an inline
Python link checker.

**Spec:** The user-approved Phase 1 audit and mapping in the `chore/reorganize-docs` task.

## Global Constraints

- Do not touch files outside `docs/` except to update references to moved docs.
- Use `git mv` for every move or rename.
- Do not delete any document.
- Keep the folder depth at two levels below `docs/`.
- Use lowercase ASCII kebab-case names without dates, version numbers, spaces,
  `new`, `final`, `misc`, or `temp`.
- Preserve `README.md`, `CONVENTIONS.md`, and `_archive/` as intentional naming
  exceptions.
- Do not rewrite document content beyond H1s, one-sentence summaries, and link
  fixes.
- If existing content is suspect, record it for review rather than changing it
  silently.

## Review Focus

- A moved document is omitted from the approved mapping or overwrites another
  target path.
- A relative Markdown, image, reference-style, or fragment link is left aimed at
  an old path.
- An outside reference in guidance, book files, crate README files, Rust docs,
  scripts, or CI is not updated.
- A moved document has an incorrect H1 count or lacks a one-sentence summary.
- The index omits an active or archived document.

---

## Task 1: Establish the worktree and record the approved plan

**Files:**

- Create: `docs/plans/chore-reorganize-docs.md`

**Interfaces:**

- Consumes: the user-approved Phase 1 mapping.
- Produces: a committed self-contained plan and a clean implementation baseline.

- [x] **Step 1: Confirm the worktree starts from `origin/main`**

Run:

```bash
git rev-parse HEAD
git rev-parse origin/main
git status --short --branch --ignore-submodules=all
```

Expected: both revisions match and the worktree is clean.

- [x] **Step 2: Run the documentation-only baseline**

Run:

```bash
scripts/agent-validation.sh --docs-only
```

Expected: exit 0.

- [x] **Step 3: Commit the plan**

```bash
git add docs/plans/chore-reorganize-docs.md
git commit -m "docs: plan documentation reorganization"
```

## Task 2: Move and rename every document

**Files:**

- Move: all 104 existing Markdown files according to the mapping below.
- Create: only the directories implied by those target paths.

**Interfaces:**

- Consumes: the approved mapping.
- Produces: a complete move-only commit whose tree contains every old document
  at exactly one new path.

- [x] **Step 1: Validate the source inventory before moving**

Run:

```bash
git ls-tree -r --name-only HEAD docs | sort
```

Expected: 105 Markdown files, including this plan, and no mapping source has
already moved.

- [x] **Step 2: Move every mapped file with `git mv`**

Use the exact pairs below. Do not use ordinary `mv`, and do not change file
contents in this task.

```text
docs/architecture.md -> docs/architecture/overview.md
docs/decoder/architecture.md -> docs/architecture/decoder.md
docs/decoder/benchmark.md -> docs/decoder/benchmark.md
docs/decoder/corpus.md -> docs/decoder/corpus.md
docs/decoder/formats/canon-cr3.md -> docs/decoder/formats/canon-cr3.md
docs/decoder/formats/dng.md -> docs/decoder/formats/dng.md
docs/decoder/formats/fuji-raf.md -> docs/decoder/formats/fuji-raf.md
docs/decoder/formats/hasselblad-3fr.md -> docs/decoder/formats/hasselblad-3fr.md
docs/decoder/formats/nikon-nef.md -> docs/decoder/formats/nikon-nef.md
docs/decoder/formats/other.md -> docs/decoder/formats/other-formats.md
docs/decoder/formats/panasonic-rw2.md -> docs/decoder/formats/panasonic-rw2.md
docs/decoder/formats/phase-one-iiq.md -> docs/decoder/formats/phase-one-iiq.md
docs/decoder/formats/sony-arw.md -> docs/decoder/formats/sony-arw.md
docs/plans/agents-md-revision.md -> docs/_archive/plans/agents-guidance-revision.md
docs/plans/amaze-implement.md -> docs/_archive/plans/amaze-port.md
docs/plans/book-01-bootstrap.md -> docs/_archive/plans/book-bootstrap.md
docs/plans/book-02-figures.md -> docs/_archive/plans/book-figures.md
docs/plans/book-03-build.md -> docs/plans/book-03-pdf-build.md
docs/plans/chore-agent-guidance-refresh.md -> docs/_archive/plans/agent-guidance-refresh.md
docs/plans/chore-cheap-golden-ci.md -> docs/_archive/plans/golden-corpus-ci.md
docs/plans/chore-ci-change-detection.md -> docs/_archive/plans/ci-change-detection.md
docs/plans/chore-codec-enum-api.md -> docs/_archive/plans/codec-enum-api.md
docs/plans/chore-codec-lzw-packbits.md -> docs/_archive/plans/lzw-packbits-codecs.md
docs/plans/chore-codec-sample-buffer.md -> docs/_archive/plans/codec-sample-buffer.md
docs/plans/chore-codec-writer-perf.md -> docs/_archive/plans/codec-writer-performance.md
docs/plans/chore-compare-demosaic-algorithm-plans.md -> docs/plans/compare-demosaic-algorithms.md
docs/plans/chore-compression-id-audit.md -> docs/_archive/workspace-refactor/compression-id-audit-implementation.md
docs/plans/chore-decoder-benchmark-devtool.md -> docs/_archive/plans/decoder-benchmark-controls.md
docs/plans/chore-decoder-tracker-agent-guide.md -> docs/_archive/plans/decoder-tracker-agent-guide.md
docs/plans/chore-demosaic-parameter-audit.md -> docs/_archive/plans/demosaic-parameter-audit.md
docs/plans/chore-dng-compression-coverage.md -> docs/_archive/plans/dng-compression-coverage.md
docs/plans/chore-dng-owned-constants.md -> docs/_archive/workspace-refactor/dng-constants-implementation.md
docs/plans/chore-dng-raw-ifd-compat-performance.md -> docs/_archive/plans/dng-raw-ifd-compatibility.md
docs/plans/chore-dng-zstd-writer-bench.md -> docs/_archive/plans/dng-zstd-writer-benchmarks.md
docs/plans/chore-extract-fornax-codec.md -> docs/_archive/plans/fornax-codec-extraction.md
docs/plans/chore-fornax-cli.md -> docs/_archive/plans/fornax-cli.md
docs/plans/chore-fornax-decoder-scaffold.md -> docs/_archive/plans/fornax-decoder-scaffold.md
docs/plans/chore-merge-bench-crates.md -> docs/_archive/plans/benchmark-crate-merge.md
docs/plans/chore-merge-pre-post-pipeline.md -> docs/_archive/workspace-refactor/raw-pipeline-merge.md
docs/plans/chore-move-demosaic-protocol-types.md -> docs/_archive/plans/demosaic-protocol-types.md
docs/plans/chore-multiband-note.md -> docs/_archive/workspace-refactor/multiband-container-record.md
docs/plans/chore-pipeline-preview-color.md -> docs/_archive/plans/dng-output-color.md
docs/plans/chore-publish-readiness.md -> docs/_archive/workspace-refactor/publish-readiness-implementation.md
docs/plans/chore-rawtherapee-demosaic-reference.md -> docs/_archive/plans/rawtherapee-demosaic-reference.md
docs/plans/chore-remove-flashraw-codecs.md -> docs/_archive/plans/flashraw-codec-removal.md
docs/plans/chore-simplify-codec-dependencies.md -> docs/_archive/plans/codec-api-simplification.md
docs/plans/chore-tiff-dng-crates.md -> docs/_archive/plans/tiff-dng-crate-split.md
docs/plans/chore-tiff-generic-accessors.md -> docs/_archive/workspace-refactor/tiff-accessors-implementation.md
docs/plans/chore-tiff-raster-feature.md -> docs/_archive/workspace-refactor/tiff-raster-feature.md
docs/plans/chore-tiff-raster.md -> docs/_archive/workspace-refactor/tiff-raster-implementation.md
docs/plans/chore-workspace-refactor-baseline.md -> docs/_archive/workspace-refactor/baseline-plan.md
docs/plans/chore-workspace-refactor-phase-a.md -> docs/_archive/workspace-refactor/phase-a.md
docs/plans/dcb-faithful-port.md -> docs/_archive/plans/dcb-port.md
docs/plans/dec-arw-sony-tiff.md -> docs/_archive/plans/sony-arw-decoder.md
docs/plans/dec-corpus-lfs-tracker.md -> docs/_archive/plans/decoder-corpus-tracker.md
docs/plans/dec-cr3-canon-cr3.md -> docs/_archive/plans/canon-cr3-decoder.md
docs/plans/dec-dng-roundtrip.md -> docs/_archive/plans/dng-roundtrip.md
docs/plans/dec-iiq-phase-one-iiq.md -> docs/_archive/plans/phase-one-iiq-decoder.md
docs/plans/dec-nef-huffman.md -> docs/_archive/plans/nikon-nef-huffman.md
docs/plans/dec-nef-packed.md -> docs/_archive/plans/nikon-nef-packed.md
docs/plans/dec-raf-fujifilm-raf.md -> docs/_archive/plans/fujifilm-raf-decoder.md
docs/plans/dec-rw2-panasonic-rw2.md -> docs/_archive/plans/panasonic-rw2-decoder.md
docs/plans/dht-faithful-port.md -> docs/_archive/plans/dht-port.md
docs/plans/lmmse-faithful-port.md -> docs/_archive/plans/lmmse-port.md
docs/plans/vng-faithful-port.md -> docs/_archive/plans/vng-port.md
docs/plans/vng4-native.md -> docs/_archive/plans/vng4.md
docs/plans/workspace-refactor/README.md -> docs/_archive/workspace-refactor/workspace-refactor-steps.md
docs/plans/workspace-refactor/b1-optional-tiff-raster.md -> docs/_archive/workspace-refactor/optional-tiff-raster-plan.md
docs/plans/workspace-refactor/b2-compression-id-audit.md -> docs/_archive/workspace-refactor/compression-id-audit-plan.md
docs/plans/workspace-refactor/b3-lzw-packbits.md -> docs/_archive/workspace-refactor/lzw-packbits-plan.md
docs/plans/workspace-refactor/b4-multiband-note.md -> docs/_archive/workspace-refactor/multiband-design-plan.md
docs/plans/workspace-refactor/c-publish-readiness.md -> docs/_archive/workspace-refactor/publish-readiness-plan.md
docs/plans/workspace-refactor/multiband-note.md -> docs/_archive/workspace-refactor/multiband-container-options.md
docs/plans/workspace-refactor/phase-b-c-handoff.md -> docs/_archive/workspace-refactor/phase-b-c-handoff.md
docs/plans/workspace-refactor/publish-readiness.md -> docs/_archive/workspace-refactor/publish-readiness-report.md
docs/plans/workspace-refactor/step-01-decode-baseline.md -> docs/_archive/workspace-refactor/decode-baseline-step.md
docs/plans/workspace-refactor/step-02-raw-pipeline.md -> docs/_archive/workspace-refactor/raw-pipeline-merge-step.md
docs/plans/workspace-refactor/step-03-tiff-accessors.md -> docs/_archive/workspace-refactor/tiff-accessors-step.md
docs/plans/workspace-refactor/step-04-sample-buffer.md -> docs/_archive/workspace-refactor/codec-sample-buffer-step.md
docs/plans/workspace-refactor/step-05-tiff-raster.md -> docs/_archive/workspace-refactor/tiff-raster-step.md
docs/plans/workspace-refactor/step-06-dng-constants.md -> docs/_archive/workspace-refactor/dng-constants-step.md
docs/proposal/steps-1-6-review-summary.md -> docs/_archive/workspace-refactor/steps-1-6-review.md
docs/references/aahd.md -> docs/reference/algorithms/aahd.md
docs/references/amaze.md -> docs/reference/algorithms/amaze.md
docs/references/canon-crx.md -> docs/reference/decoders/canon-crx.md
docs/references/compression-ids.md -> docs/reference/codecs/compression-ids.md
docs/references/dcb.md -> docs/reference/algorithms/dcb.md
docs/references/dht.md -> docs/reference/algorithms/dht.md
docs/references/dng-compression.md -> docs/reference/decoders/dng-compression.md
docs/references/dng-lossless-jpeg.md -> docs/reference/decoders/dng-lossless-jpeg.md
docs/references/dng-opcodes.md -> docs/reference/decoders/dng-opcodes.md
docs/references/dng-preview-color.md -> docs/reference/decoders/dng-output-color.md
docs/references/fuji-raf.md -> docs/reference/decoders/fuji-raf.md
docs/references/lmmse.md -> docs/reference/algorithms/lmmse.md
docs/references/nikon-nef.md -> docs/reference/decoders/nikon-nef.md
docs/references/panasonic-rw2.md -> docs/reference/decoders/panasonic-rw2.md
docs/references/phase-one-iiq.md -> docs/reference/decoders/phase-one-iiq.md
docs/references/rawtherapee.md -> docs/reference/algorithms/rawtherapee.md
docs/references/sony-arw.md -> docs/reference/decoders/sony-arw.md
docs/references/vng.md -> docs/reference/algorithms/vng.md
docs/references/workspace-refactor-baseline.md -> docs/_archive/workspace-refactor/baseline-findings.md
docs/references/workspace-refactor-measurements.md -> docs/_archive/workspace-refactor/measurements.md
docs/superpowers/specs/2026-11-03-dcb-faithful-port-design.md -> docs/_archive/designs/dcb-port-design.md
docs/superpowers/specs/2026-11-04-dht-faithful-port-design.md -> docs/_archive/designs/dht-port-design.md
```

- [x] **Step 3: Verify the move-only commit**

Run:

```bash
git diff --summary --cached
git diff --name-only --cached | wc -l
```

Expected: only rename records appear, and the old-to-new path count matches the
approved mapping.

- [x] **Step 4: Commit the moves**

```bash
git add -A docs
git commit -m "docs: reorganize documentation tree"
```

## Task 3: Repair links and normalize document openings

**Files:**

- Modify: moved Markdown files under `docs/`.
- Modify outside references in `AGENTS.md`, `TODO.md`, `.agents/`, `book/`,
  crate README files, and Rust documentation comments.

**Interfaces:**

- Consumes: the move-only commit from Task 2.
- Produces: zero stale documentation paths and valid relative links/anchors.

- [x] **Step 1: Update every internal link and fragment**

Resolve links relative to each document's new location, including Markdown
links, images, and reference definitions.

- [x] **Step 2: Update every outside reference to moved documents**

Update references in:

- `AGENTS.md`
- `TODO.md`
- `.agents/skills/decoder-corpus-tracker/SKILL.md`
- `book/AGENTS.md`
- `book/README.md`
- `book/template.md`
- `book/src/bibliography.md`
- `book/src/glossary.md`
- `book/src/notation.md`
- `crates/compare-demosaic/README.md`
- `crates/decoder-tracker/AGENTS.md`
- `crates/decoder-tracker/README.md`
- `crates/fornax-codec/README.md`
- `crates/fornax-codec/src/ljpeg.rs`
- `crates/fornax-codec/src/nef.rs`
- `crates/fornax-dng/src/constants.rs`
- `crates/fornax-iiq/README.md`
- `crates/fornax-raf/README.md`
- `crates/fornax-raw-pipeline/src/demosaic/aahd.rs`
- `crates/fornax-raw-pipeline/src/demosaic/amaze.rs`
- `crates/fornax-raw-pipeline/src/demosaic/dcb.rs`
- `crates/fornax-raw-pipeline/src/demosaic/lmmse.rs`
- `crates/fornax-raw-pipeline/src/demosaic/vng.rs`
- `crates/fornax-raw-py/README.md`
- `crates/fornax-rw2/README.md`
- `crates/fornax-tiff/README.md`

- [x] **Step 3: Normalize headings and summaries**

For every moved document:

- retain exactly one H1;
- make the H1 describe the document rather than its old workflow prefix;
- place one complete sentence immediately after the H1.

Do not change other prose except to repair links.

- [x] **Step 4: Run the relative-link checker**

Check Markdown links, images, reference definitions, and local fragments across
all of `docs/`, plus all references from outside `docs/`.

Expected: zero broken relative targets and zero broken local anchors.

- [x] **Step 5: Commit link and heading repairs**

```bash
git add AGENTS.md TODO.md .agents book crates docs
git commit -m "docs: repair links after documentation reorganization"
```

## Task 4: Add the documentation index and conventions

**Files:**

- Create: `docs/README.md`
- Create: `docs/CONVENTIONS.md`

**Interfaces:**

- Consumes: the repaired tree from Task 3.
- Produces: an index linking to every other document with one-line summaries and
  rules for future placement and naming.

- [x] **Step 1: Create `docs/README.md`**

Describe each top-level section and link every other Markdown document,
including archived documents, with a one-line summary.

- [x] **Step 2: Create `docs/CONVENTIONS.md`**

Document:

- the maximum two-folder hierarchy;
- the purpose of `architecture/`, `decoder/`, `reference/`, `plans/`, and
  `_archive/`;
- lowercase ASCII kebab-case naming;
- the `README.md`, `CONVENTIONS.md`, and `_archive/` exceptions;
- where new architecture, decoder format, algorithm, codec, plan, decision, and
  reference documents belong;
- ADR naming as `decisions/0001-title.md` if that folder is later added.

- [x] **Step 3: Verify index completeness**

Compare the set of Markdown files under `docs/` against the relative links in
`docs/README.md`.

Expected: every document except `docs/README.md` is linked.

- [x] **Step 4: Run documentation validation**

```bash
scripts/agent-validation.sh --docs-only
git diff --check
```

Expected: both commands exit 0.

- [x] **Step 5: Commit the index and conventions**

```bash
git add docs/README.md docs/CONVENTIONS.md
git commit -m "docs: add documentation index and conventions"
```

## Task 5: Final verification and review

**Files:**

- Modify only if validation finds a defect.

**Interfaces:**

- Consumes: all prior tasks.
- Produces: a validated branch, final audit summary, and pull request.

- [ ] **Step 1: Re-run all structural checks**

Verify:

- exactly one H1 per document;
- every H1 is the first non-blank line;
- every document has a non-empty summary paragraph;
- every old path is absent;
- every approved new path exists;
- every relative link and fragment resolves;
- every document is represented in the index;
- no stale `docs/references/`, `docs/proposal/`, `docs/superpowers/`, or moved
  plan paths remain outside the archive.

Expected: every check passes with zero findings.

- [ ] **Step 2: Run repository documentation validation**

```bash
scripts/agent-validation.sh --docs-only
git diff --check
```

Expected: exit 0.

- [ ] **Step 3: Inspect the complete diff and commit log**

Confirm the only non-documentation changes are reference repairs, and confirm
the commit sequence contains the plan, moves, link repairs, and index.

- [ ] **Step 4: Perform the final whole-branch self-review**

Read the diff against this plan and the approved audit, using the Review Focus
items as an explicit checklist.

- [ ] **Step 5: Push and open one semantic pull request**

```bash
git push -u origin HEAD
gh pr create --base main \
  --title "docs: reorganize documentation tree" \
  --body-file /tmp/fornax-docs-reorg-pr.md
```

Expected: the branch is pushed and one PR URL is returned.

## Validation

- `scripts/agent-validation.sh --docs-only`
- Relative Markdown/image/reference/anchor checker over all Markdown files.
- Index completeness check.
- H1 and summary structure check.
- Approved mapping existence and uniqueness check.
- `git diff --check`.
- Manual whole-branch review.

## Decisions and notes

- Completed plans and workspace-refactor evidence are archived rather than
  merged because they preserve distinct design, execution, and result roles.
- No document is deleted.
- `docs/plans/` remains the active-plan folder because root `AGENTS.md` already
  defines that workflow.
- The decoder benchmark’s current coverage wording and completed plans’
  checkbox inconsistencies are recorded as content issues, not silently fixed.
- There is no docs-site build; documentation validation is
  `scripts/agent-validation.sh --docs-only` plus the link/structure checks above.
