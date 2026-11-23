# Archive and Clean Stale Documentation

This plan inventories the current `docs/` tree, protects the DNG performance
resume material, and proposes the docs-only cleanup for review.

## Goal

Make `docs/` easy to navigate without losing completed-plan provenance,
negative performance results, reproduction instructions, or the source needed
to resume DNG work.

## Scope

In scope:

- Inventory every current file under `docs/` with evidence-backed provenance.
- Preserve the DNG performance resume pack without delete actions.
- Normalize the existing archive root to `docs/archive/`.
- Archive completed plans and superseded investigation reports.
- Archive generated profile artifacts while retaining every protected DNG
  experiment artifact.
- Update links, documentation conventions, and the current docs index.
- Add a dependency-free link check for CI if the repository still has no
  equivalent check.

Out of scope:

- Production code.
- Demosaic, decoder, or performance implementation changes.
- Deleting branches or worktrees.
- Deleting any file protected by the DNG performance resume pack.
- The separate large-allocation pipeline audit, which has its own branch and PR.

## Current Snapshot

- Before this planning branch: 179 tracked files under `docs/`, 2.3 MiB.
- Current working tree: 182 files after adding the two protected resume-pack
  files and this active plan.
- Existing archive: 72 files, 756 KiB.
- Current broken Markdown links: 0.
- Tracked binary media under `docs/`: none matched by the requested extension check.
- Proposed action counts: archive: 36, archive root rename: 72, keep: 70, keep + trim: 1, keep + update archive path: 1, keep + update index: 1.

## Review Decisions

Action meanings:

- `keep`: leave in the current documentation tree.
- `keep + trim`: retain as current documentation, remove stale wording, and
  redirect historical detail to the protected experiment log.
- `keep + update index`: retain but rewrite its navigation/listing content.
- `archive root rename`: `git mv docs/_archive docs/archive`; relocations
  inside the tree preserve relative links.
- `archive`: `git mv` to `docs/archive/<same subpath>`, prepend the required
  three-line status/date/current-document header to Markdown documents, and add
  a one-line entry to `docs/archive/README.md`.
- No `delete` action is proposed. Total bytes removed: 0.

## Execution Result

- Renamed the existing 72-file archive root from `docs/_archive/` to
  `docs/archive/`.
- Archived 36 additional files: 16 completed plans, the superseded
  `dng-performance.md`, and 19 generated profile artifacts.
- Added three-line status headers to the 17 newly archived Markdown files.
- Added a 108-entry `docs/archive/README.md` index.
- Deleted nothing. Total bytes removed: 0.
- `docs/` now contains 183 files and is 2.4 MiB including this active plan.
- The DNG performance resume pack remains current and protected.
- Added `scripts/check-doc-links.py`, a CI documentation-link job, and an
  agent-validation test/hook for the checker.
- `python3 scripts/check-doc-links.py` passes.
- `bash scripts/agent-validation-test.sh` passes.
- `scripts/agent-validation.sh --docs-only` passes.
- The requested binary-extension scan returns no files.

FlashRaw 0.2.0 is MIT OR Apache-2.0. The comparison note records ideas and
source paths; no FlashRaw code was copied.

## File Inventory and Proposal

| Path | Bytes | Last commit | Introduced | Status | Action | Reason | Inbound links |
| --- | ---: | --- | --- | --- | --- | --- | --- |
| `docs/CONVENTIONS.md` | 2580 | 2026-11-12 | #715 `chore/reorganize-docs` | ACTIVE | keep + update archive path | Current documentation policy; align it with docs/archive | docs/README.md:16<br>docs/README.md:183 |
| `docs/README.md` | 21910 | 2026-11-15 | #715 `chore/reorganize-docs` | ACTIVE | keep + update index | Current documentation entry point; reduce the archive listing to a pointer and index the new DNG documents | - |
| `docs/_archive/designs/dcb-port-design.md` | 7985 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:176<br>docs/_archive/plans/dcb-port.md:23 |
| `docs/_archive/designs/dht-port-design.md` | 5741 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:177<br>docs/_archive/plans/dht-port.md:23 |
| `docs/_archive/plans/agent-guidance-refresh.md` | 5964 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:96 |
| `docs/_archive/plans/agents-guidance-revision.md` | 10864 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:97 |
| `docs/_archive/plans/amaze-port.md` | 10843 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:98 |
| `docs/_archive/plans/benchmark-crate-merge.md` | 14088 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:99 |
| `docs/_archive/plans/book-bootstrap.md` | 11228 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:100 |
| `docs/_archive/plans/book-figures.md` | 5840 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:101 |
| `docs/_archive/plans/canon-cr3-decoder.md` | 11229 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:102 |
| `docs/_archive/plans/ci-change-detection.md` | 2606 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:103 |
| `docs/_archive/plans/codec-api-simplification.md` | 7605 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:104 |
| `docs/_archive/plans/codec-enum-api.md` | 8442 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:105 |
| `docs/_archive/plans/codec-sample-buffer.md` | 8207 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:106 |
| `docs/_archive/plans/codec-writer-performance.md` | 4402 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:107 |
| `docs/_archive/plans/dcb-port.md` | 25898 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:108 |
| `docs/_archive/plans/decoder-benchmark-controls.md` | 6491 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:109 |
| `docs/_archive/plans/decoder-corpus-tracker.md` | 8096 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:110 |
| `docs/_archive/plans/decoder-tracker-agent-guide.md` | 4307 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:111 |
| `docs/_archive/plans/demosaic-parameter-audit.md` | 12846 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:112 |
| `docs/_archive/plans/demosaic-protocol-types.md` | 10166 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:113 |
| `docs/_archive/plans/dht-port.md` | 11368 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:114 |
| `docs/_archive/plans/dng-compression-coverage.md` | 10315 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:115 |
| `docs/_archive/plans/dng-output-color.md` | 4356 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:116 |
| `docs/_archive/plans/dng-raw-ifd-compatibility.md` | 12809 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:117 |
| `docs/_archive/plans/dng-roundtrip.md` | 9356 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:118 |
| `docs/_archive/plans/dng-zstd-writer-benchmarks.md` | 4458 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:119 |
| `docs/_archive/plans/flashraw-codec-removal.md` | 5151 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:120 |
| `docs/_archive/plans/fornax-cli.md` | 5400 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:121 |
| `docs/_archive/plans/fornax-codec-extraction.md` | 12002 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:122 |
| `docs/_archive/plans/fornax-decoder-scaffold.md` | 8204 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:123 |
| `docs/_archive/plans/fujifilm-raf-decoder.md` | 6993 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:124 |
| `docs/_archive/plans/golden-corpus-ci.md` | 3014 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:125 |
| `docs/_archive/plans/hasselblad-3fr-decoder.md` | 9447 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:126 |
| `docs/_archive/plans/lmmse-port.md` | 3051 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:127 |
| `docs/_archive/plans/lzw-packbits-codecs.md` | 7608 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:128 |
| `docs/_archive/plans/nikon-nef-huffman.md` | 13924 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:129 |
| `docs/_archive/plans/nikon-nef-packed.md` | 6383 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:130 |
| `docs/_archive/plans/panasonic-rw2-decoder.md` | 13139 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:131 |
| `docs/_archive/plans/phase-one-iiq-decoder.md` | 10915 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:132 |
| `docs/_archive/plans/rawtherapee-demosaic-reference.md` | 13493 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:133 |
| `docs/_archive/plans/sony-arw-decoder.md` | 5585 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:134 |
| `docs/_archive/plans/tiff-dng-crate-split.md` | 7600 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:135 |
| `docs/_archive/plans/vng-port.md` | 5269 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:136 |
| `docs/_archive/plans/vng4.md` | 5922 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:137 |
| `docs/_archive/workspace-refactor/baseline-findings.md` | 6553 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:143<br>docs/_archive/workspace-refactor/decode-baseline-step.md:24<br>docs/_archive/workspace-refactor/workspace-refactor-steps.md:43 |
| `docs/_archive/workspace-refactor/baseline-plan.md` | 5164 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:144 |
| `docs/_archive/workspace-refactor/codec-sample-buffer-step.md` | 2744 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:145<br>docs/_archive/workspace-refactor/workspace-refactor-steps.md:18 |
| `docs/_archive/workspace-refactor/compression-id-audit-implementation.md` | 6426 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:146 |
| `docs/_archive/workspace-refactor/compression-id-audit-plan.md` | 8532 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:147<br>docs/_archive/workspace-refactor/phase-b-c-handoff.md:54<br>docs/_archive/workspace-refactor/workspace-refactor-steps.md:32 |
| `docs/_archive/workspace-refactor/decode-baseline-step.md` | 830 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:148<br>docs/_archive/workspace-refactor/workspace-refactor-steps.md:15 |
| `docs/_archive/workspace-refactor/dng-constants-implementation.md` | 18083 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:149 |
| `docs/_archive/workspace-refactor/dng-constants-step.md` | 2717 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:150<br>docs/_archive/workspace-refactor/workspace-refactor-steps.md:20 |
| `docs/_archive/workspace-refactor/lzw-packbits-plan.md` | 9154 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:151<br>docs/_archive/workspace-refactor/phase-b-c-handoff.md:55<br>docs/_archive/workspace-refactor/workspace-refactor-steps.md:33 |
| `docs/_archive/workspace-refactor/measurements.md` | 4933 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:152 |
| `docs/_archive/workspace-refactor/multiband-container-options.md` | 23746 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:153<br>docs/_archive/workspace-refactor/workspace-refactor-steps.md:35 |
| `docs/_archive/workspace-refactor/multiband-container-record.md` | 3577 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:154 |
| `docs/_archive/workspace-refactor/multiband-design-plan.md` | 5143 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:155<br>docs/_archive/workspace-refactor/phase-b-c-handoff.md:56<br>docs/_archive/workspace-refactor/workspace-refactor-steps.md:34 |
| `docs/_archive/workspace-refactor/optional-tiff-raster-plan.md` | 5327 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:156<br>docs/_archive/workspace-refactor/phase-b-c-handoff.md:53<br>docs/_archive/workspace-refactor/workspace-refactor-steps.md:31 |
| `docs/_archive/workspace-refactor/phase-a.md` | 4815 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:157<br>docs/_archive/workspace-refactor/workspace-refactor-steps.md:23 |
| `docs/_archive/workspace-refactor/phase-b-c-handoff.md` | 3553 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:158<br>docs/_archive/workspace-refactor/multiband-container-options.md:363<br>docs/_archive/workspace-refactor/workspace-refactor-steps.md:30 |
| `docs/_archive/workspace-refactor/publish-readiness-implementation.md` | 8818 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:159 |
| `docs/_archive/workspace-refactor/publish-readiness-plan.md` | 6768 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:160<br>docs/_archive/workspace-refactor/phase-b-c-handoff.md:57<br>docs/_archive/workspace-refactor/workspace-refactor-steps.md:36 |
| `docs/_archive/workspace-refactor/publish-readiness-report.md` | 14641 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:161 |
| `docs/_archive/workspace-refactor/raw-pipeline-merge-step.md` | 2615 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:162<br>docs/_archive/workspace-refactor/workspace-refactor-steps.md:16 |
| `docs/_archive/workspace-refactor/raw-pipeline-merge.md` | 13373 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:163 |
| `docs/_archive/workspace-refactor/steps-1-6-review.md` | 25351 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | crates/fornax-raw-py/README.md:18<br>docs/README.md:164<br>docs/architecture/overview.md:50 |
| `docs/_archive/workspace-refactor/tiff-accessors-implementation.md` | 7058 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:165 |
| `docs/_archive/workspace-refactor/tiff-accessors-step.md` | 2242 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:166<br>docs/_archive/workspace-refactor/workspace-refactor-steps.md:17 |
| `docs/_archive/workspace-refactor/tiff-raster-feature.md` | 4469 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:167 |
| `docs/_archive/workspace-refactor/tiff-raster-implementation.md` | 20301 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:168 |
| `docs/_archive/workspace-refactor/tiff-raster-step.md` | 2475 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:169<br>docs/_archive/workspace-refactor/workspace-refactor-steps.md:19 |
| `docs/_archive/workspace-refactor/workspace-refactor-steps.md` | 3126 | 2026-11-12 | #715 `chore/reorganize-docs` | COMPLETED | archive root rename | Preserve the completed record while normalizing the archive root | docs/README.md:170 |
| `docs/architecture/decoder.md` | 17698 | 2026-11-15 | #715 `chore/reorganize-docs` | ACTIVE | keep | Current architecture, decoder, reference, or active guidance | AGENTS.md:197<br>docs/README.md:34<br>docs/_archive/plans/fornax-decoder-scaffold.md:20<br>docs/architecture/overview.md:328 |
| `docs/architecture/overview.md` | 19286 | 2026-11-15 | #715 `chore/reorganize-docs` | ACTIVE | keep | Current architecture, decoder, reference, or active guidance | AGENTS.md:12<br>AGENTS.md:196<br>book/README.md:11<br>book/README.md:70<br>book/src/bibliography.md:10<br>book/src/glossary.md:11<br>book/src/glossary.md:12<br>book/src/glossary.md:13<br>book/src/glossary.md:14<br>book/src/glossary.md:16<br>book/src/glossary.md:17<br>book/src/glossary.md:18<br>book/src/glossary.md:19<br>book/src/glossary.md:8<br>book/src/notation.md:13<br>docs/README.md:35<br>docs/README.md:7 |
| `docs/decoder/benchmark.md` | 6561 | 2026-11-15 | #631 `chore/fornax-decoder-scaffold` | ACTIVE | keep | Current architecture, decoder, reference, or active guidance | AGENTS.md:198<br>docs/README.md:41<br>docs/architecture/overview.md:329 |
| `docs/decoder/corpus.md` | 20705 | 2026-11-15 | #631 `chore/fornax-decoder-scaffold` | ACTIVE | keep | Current architecture, decoder, reference, or active guidance | .agents/skills/decoder-corpus-tracker/SKILL.md:10<br>AGENTS.md:199<br>crates/decoder-tracker/AGENTS.md:10<br>crates/decoder-tracker/AGENTS.md:169<br>crates/decoder-tracker/AGENTS.md:4<br>crates/decoder-tracker/README.md:168<br>docs/README.md:42<br>docs/architecture/overview.md:330 |
| `docs/decoder/formats/canon-cr3.md` | 4870 | 2026-11-12 | #631 `chore/fornax-decoder-scaffold` | ACTIVE | keep | Current architecture, decoder, reference, or active guidance | docs/README.md:43<br>docs/architecture/decoder.md:33 |
| `docs/decoder/formats/dng.md` | 8046 | 2026-11-12 | #631 `chore/fornax-decoder-scaffold` | ACTIVE | keep | Current architecture, decoder, reference, or active guidance | docs/README.md:44<br>docs/architecture/decoder.md:32 |
| `docs/decoder/formats/fuji-raf.md` | 2755 | 2026-11-12 | #631 `chore/fornax-decoder-scaffold` | ACTIVE | keep | Current architecture, decoder, reference, or active guidance | docs/README.md:45<br>docs/architecture/decoder.md:36 |
| `docs/decoder/formats/hasselblad-3fr.md` | 3774 | 2026-11-12 | #631 `chore/fornax-decoder-scaffold` | ACTIVE | keep | Current architecture, decoder, reference, or active guidance | docs/README.md:46<br>docs/architecture/decoder.md:37 |
| `docs/decoder/formats/nikon-nef.md` | 4070 | 2026-11-12 | #631 `chore/fornax-decoder-scaffold` | ACTIVE | keep | Current architecture, decoder, reference, or active guidance | docs/README.md:47<br>docs/architecture/decoder.md:35 |
| `docs/decoder/formats/other-formats.md` | 822 | 2026-11-12 | #631 | ACTIVE | keep | Current architecture, decoder, reference, or active guidance | docs/README.md:48<br>docs/architecture/decoder.md:40 |
| `docs/decoder/formats/panasonic-rw2.md` | 4935 | 2026-11-12 | #631 `chore/fornax-decoder-scaffold` | ACTIVE | keep | Current architecture, decoder, reference, or active guidance | crates/fornax-raw-panasonic/README.md:29<br>docs/README.md:49<br>docs/architecture/decoder.md:251<br>docs/architecture/decoder.md:38 |
| `docs/decoder/formats/phase-one-iiq.md` | 4943 | 2026-11-12 | #631 `chore/fornax-decoder-scaffold` | ACTIVE | keep | Current architecture, decoder, reference, or active guidance | docs/README.md:50<br>docs/architecture/decoder.md:39 |
| `docs/decoder/formats/sony-arw.md` | 5238 | 2026-11-12 | #631 `chore/fornax-decoder-scaffold` | ACTIVE | keep | Current architecture, decoder, reference, or active guidance | docs/README.md:51<br>docs/architecture/decoder.md:34 |
| `docs/plans/book-03-pdf-build.md` | 5716 | 2026-11-12 | #622 | ACTIVE | keep | Named by TODO.md or still active comparison work | TODO.md:15<br>docs/README.md:57 |
| `docs/plans/chore-arw-gap-profiles.md` | 3753 | 2026-11-15 | #735 `chore/arw-gap-profiles` | COMPLETED | archive | Merged PR; the plan is historical rather than active guidance | - |
| `docs/plans/chore-close-dng-flashraw-gap.md` | 18002 | 2026-11-15 | #724 `chore/close-dng-flashraw-gap` | COMPLETED | archive | Merged PR; the plan is historical rather than active guidance | - |
| `docs/plans/chore-decoder-benchmark-golden-corpus.md` | 9897 | 2026-11-12 | #717 | COMPLETED | archive | Merged PR; the plan is historical rather than active guidance | - |
| `docs/plans/chore-dng-first-touch-probe.md` | 4867 | 2026-11-15 | #727 `chore/dng-first-touch-probe` | COMPLETED | archive | Merged PR; the plan is historical rather than active guidance | - |
| `docs/plans/chore-dng-followup-probes.md` | 9488 | 2026-11-15 | #728 `chore/dng-followup-probes` | COMPLETED | archive | Merged PR; the plan is historical rather than active guidance | - |
| `docs/plans/chore-encode-image-fast-path.md` | 3601 | 2026-11-15 | #725 `chore/encode-image-fast-path` | COMPLETED | archive | Merged PR; the plan is historical rather than active guidance | - |
| `docs/plans/chore-flashraw-cpu-followup.md` | 9812 | 2026-11-16 | #738 `chore/large-page-allocation` | COMPLETED | archive | Merged PR; the plan is historical rather than active guidance | docs/plans/chore-large-page-allocation.md:218 |
| `docs/plans/chore-full-corpus-metadata.md` | 14804 | 2026-11-14 | #722 `chore/full-corpus-metadata` | COMPLETED | archive | Merged PR; the plan is historical rather than active guidance | - |
| `docs/plans/chore-large-page-allocation.md` | 11188 | 2026-11-15 | #738 `chore/large-page-allocation` | COMPLETED | archive | Merged PR; the plan is historical rather than active guidance | - |
| `docs/plans/chore-ljpeg-critical-path.md` | 8283 | 2026-11-15 | #737 `chore/ljpeg-critical-path` | COMPLETED | archive | Merged PR; the plan is historical rather than active guidance | - |
| `docs/plans/chore-ljpeg-fault-followup.md` | 7716 | 2026-11-15 | #730 `chore/ljpeg-fault-followup` | COMPLETED | archive | Merged PR; the plan is historical rather than active guidance | - |
| `docs/plans/chore-ljpeg-instruction-gap.md` | 28357 | 2026-11-15 | #736 `chore/ljpeg-instruction-gap` | COMPLETED | archive | Merged PR; the plan is historical rather than active guidance | - |
| `docs/plans/chore-minimal-dng-golden.md` | 3349 | 2026-11-15 | #723 `chore/minimal-dng-golden` | COMPLETED | archive | Merged PR; the plan is historical rather than active guidance | - |
| `docs/plans/chore-portable-scalar-codec.md` | 19065 | 2026-11-15 | #734 `chore/portable-scalar-codec` | COMPLETED | archive | Merged PR; the plan is historical rather than active guidance | - |
| `docs/plans/chore-reorganize-docs.md` | 19985 | 2026-11-12 | #715 | COMPLETED | archive | Merged PR; the plan is historical rather than active guidance | docs/README.md:58 |
| `docs/plans/compare-demosaic-algorithms.md` | 45394 | 2026-11-12 | #618 | ACTIVE | keep | Named by TODO.md or still active comparison work | docs/README.md:59 |
| `docs/plans/perf-dng-decode.md` | 42170 | 2026-11-14 | #720 `task/perf-dng-decode` | COMPLETED | archive | Merged PR; the plan is historical rather than active guidance | docs/reference/decoders/dng-performance.md:6<br>docs/reference/decoders/perf/dng-remaining-gap.md:4 |
| `docs/reference/algorithms/aahd.md` | 2619 | 2026-11-12 | #634 | ACTIVE | keep | Current architecture, decoder, reference, or active guidance | docs/README.md:65 |
| `docs/reference/algorithms/amaze.md` | 3204 | 2026-11-12 | #640 | ACTIVE | keep | Current architecture, decoder, reference, or active guidance | docs/README.md:66 |
| `docs/reference/algorithms/dcb.md` | 4764 | 2026-11-12 | #636 | ACTIVE | keep | Current architecture, decoder, reference, or active guidance | docs/README.md:67 |
| `docs/reference/algorithms/dht.md` | 4357 | 2026-11-12 | #639 | ACTIVE | keep | Current architecture, decoder, reference, or active guidance | docs/README.md:68 |
| `docs/reference/algorithms/lmmse.md` | 3890 | 2026-11-12 | #635 | ACTIVE | keep | Current architecture, decoder, reference, or active guidance | docs/README.md:69 |
| `docs/reference/algorithms/rawtherapee.md` | 4702 | 2026-11-12 | #637 | ACTIVE | keep | Current architecture, decoder, reference, or active guidance | docs/README.md:70 |
| `docs/reference/algorithms/vng.md` | 4901 | 2026-11-12 | #619 | ACTIVE | keep | Current architecture, decoder, reference, or active guidance | book/src/bibliography.md:26<br>book/src/notation.md:31<br>crates/compare-demosaic/README.md:572<br>docs/README.md:71 |
| `docs/reference/codecs/compression-ids.md` | 4712 | 2026-11-12 | #695 | ACTIVE | keep | Current architecture, decoder, reference, or active guidance | crates/fornax-tiff/README.md:74<br>docs/README.md:72<br>docs/reference/codecs/lossless-classification.md:12<br>docs/reference/decoders/reference/dng-compression.md:10<br>docs/reference/decoders/reference/dng-compression.md:55 |
| `docs/reference/codecs/lossless-classification.md` | 2504 | 2026-11-14 | #722 `chore/full-corpus-metadata` | ACTIVE | keep | Current architecture, decoder, reference, or active guidance | docs/README.md:73<br>docs/decoder/corpus.md:258 |
| `docs/reference/decoders/perf/arw-gap-profiles/exiftool-selected.json` | 2836 | 2026-11-15 | #735 `chore/arw-gap-profiles` | ACTIVE | keep | Evidence for the current ARW gap report | docs/reference/decoders/perf/arw-remaining-gap.md:88 |
| `docs/reference/decoders/perf/arw-gap-profiles/inventory.csv` | 26746 | 2026-11-15 | #735 `chore/arw-gap-profiles` | ACTIVE | keep | Evidence for the current ARW gap report | docs/reference/decoders/perf/arw-remaining-gap.md:217<br>docs/reference/decoders/perf/arw-remaining-gap.md:74 |
| `docs/reference/decoders/perf/arw-gap-profiles/perf-stat.txt` | 1954 | 2026-11-15 | #735 `chore/arw-gap-profiles` | ACTIVE | keep | Evidence for the current ARW gap report | docs/reference/decoders/perf/arw-remaining-gap.md:130 |
| `docs/reference/decoders/perf/arw-gap-profiles/stages.txt` | 1894 | 2026-11-15 | #735 `chore/arw-gap-profiles` | ACTIVE | keep | Evidence for the current ARW gap report | docs/reference/decoders/perf/arw-remaining-gap.md:147 |
| `docs/reference/decoders/perf/arw-gap-profiles/stream-verification.txt` | 2211 | 2026-11-15 | #735 `chore/arw-gap-profiles` | ACTIVE | keep | Evidence for the current ARW gap report | docs/reference/decoders/perf/arw-remaining-gap.md:71 |
| `docs/reference/decoders/perf/arw-gap-profiles/timings.txt` | 2037 | 2026-11-15 | #735 `chore/arw-gap-profiles` | ACTIVE | keep | Evidence for the current ARW gap report | docs/reference/decoders/perf/arw-remaining-gap.md:111 |
| `docs/reference/decoders/perf/arw-remaining-gap.md` | 12923 | 2026-11-15 | #735 `chore/arw-gap-profiles` | ACTIVE | keep | Current architecture, decoder, reference, or active guidance | docs/README.md:82<br>docs/reference/decoders/perf/dng-gap-profiles/RESUME.md:84 |
| `docs/reference/decoders/reference/canon-crx.md` | 2958 | 2026-11-12 | #711 | ACTIVE | keep | Current architecture, decoder, reference, or active guidance | docs/README.md:74 |
| `docs/reference/decoders/reference/dng-compression.md` | 2667 | 2026-11-12 | #650 | ACTIVE | keep | Current architecture, decoder, reference, or active guidance | crates/fornax-raw/src/dng/constants.rs:5<br>docs/README.md:75 |
| `docs/reference/decoders/perf/dng-experiments-log.md` | 5644 | working tree | this branch (`chore/archive-stale-docs`) | ACTIVE | keep | Required DNG performance resume-pack entry | docs/README.md:79<br>docs/reference/decoders/perf/dng-gap-profiles/README.md:10<br>docs/reference/decoders/perf/dng-gap-profiles/RESUME.md:6<br>docs/reference/decoders/perf/dng-remaining-gap.md:9 |
| `docs/reference/decoders/perf/dng-gap-profiles/5d3-fp32-deflate-flashraw.folded` | 10549 | 2026-11-15 | #724 `chore/close-dng-flashraw-gap` | GENERATED/RAW | archive | Regenerable raw/profile artifact retained under the protected resume policy | - |
| `docs/reference/decoders/perf/dng-gap-profiles/5d3-fp32-deflate-flashraw.svg` | 9290 | 2026-11-15 | #724 `chore/close-dng-flashraw-gap` | GENERATED/RAW | archive | Regenerable raw/profile artifact retained under the protected resume policy | - |
| `docs/reference/decoders/perf/dng-gap-profiles/5d3-fp32-deflate-fornax.folded` | 223440 | 2026-11-15 | #724 `chore/close-dng-flashraw-gap` | GENERATED/RAW | archive | Regenerable raw/profile artifact retained under the protected resume policy | - |
| `docs/reference/decoders/perf/dng-gap-profiles/5d3-fp32-deflate-fornax.svg` | 30337 | 2026-11-15 | #724 `chore/close-dng-flashraw-gap` | GENERATED/RAW | archive | Regenerable raw/profile artifact retained under the protected resume policy | - |
| `docs/reference/decoders/perf/dng-gap-profiles/5d3-ljpeg-flashraw.folded` | 8912 | 2026-11-15 | #724 `chore/close-dng-flashraw-gap` | GENERATED/RAW | archive | Regenerable raw/profile artifact retained under the protected resume policy | - |
| `docs/reference/decoders/perf/dng-gap-profiles/5d3-ljpeg-flashraw.svg` | 8129 | 2026-11-15 | #724 `chore/close-dng-flashraw-gap` | GENERATED/RAW | archive | Regenerable raw/profile artifact retained under the protected resume policy | - |
| `docs/reference/decoders/perf/dng-gap-profiles/5d3-ljpeg-fornax.folded` | 109327 | 2026-11-15 | #724 `chore/close-dng-flashraw-gap` | GENERATED/RAW | archive | Regenerable raw/profile artifact retained under the protected resume policy | - |
| `docs/reference/decoders/perf/dng-gap-profiles/5d3-ljpeg-fornax.svg` | 25977 | 2026-11-15 | #724 `chore/close-dng-flashraw-gap` | GENERATED/RAW | archive | Regenerable raw/profile artifact retained under the protected resume policy | - |
| `docs/reference/decoders/perf/dng-gap-profiles/5d3-uncompressed-flashraw.folded` | 3838 | 2026-11-15 | #724 `chore/close-dng-flashraw-gap` | GENERATED/RAW | archive | Regenerable raw/profile artifact retained under the protected resume policy | - |
| `docs/reference/decoders/perf/dng-gap-profiles/5d3-uncompressed-flashraw.svg` | 8262 | 2026-11-15 | #724 `chore/close-dng-flashraw-gap` | GENERATED/RAW | archive | Regenerable raw/profile artifact retained under the protected resume policy | - |
| `docs/reference/decoders/perf/dng-gap-profiles/5d3-uncompressed-fornax.folded` | 6907 | 2026-11-15 | #724 `chore/close-dng-flashraw-gap` | GENERATED/RAW | archive | Regenerable raw/profile artifact retained under the protected resume policy | - |
| `docs/reference/decoders/perf/dng-gap-profiles/5d3-uncompressed-fornax.svg` | 13552 | 2026-11-15 | #724 `chore/close-dng-flashraw-gap` | GENERATED/RAW | archive | Regenerable raw/profile artifact retained under the protected resume policy | - |
| `docs/reference/decoders/perf/dng-gap-profiles/README.md` | 27312 | 2026-11-16 | #736 `chore/ljpeg-instruction-gap` | ACTIVE | keep | Current detailed evidence or summary table retained by the protected resume pack | docs/reference/decoders/perf/dng-experiments-log.md:7<br>docs/reference/decoders/perf/dng-remaining-gap.md:610 |
| `docs/reference/decoders/perf/dng-gap-profiles/RESUME.md` | 4630 | working tree | this branch (`chore/archive-stale-docs`) | ACTIVE | keep | Required DNG performance resume-pack entry | docs/README.md:81<br>docs/reference/decoders/perf/dng-experiments-log.md:8<br>docs/reference/decoders/perf/dng-gap-profiles/README.md:8<br>docs/reference/decoders/perf/dng-remaining-gap.md:11 |
| `docs/reference/decoders/perf/dng-gap-profiles/deflate-minor-faults.txt` | 831 | 2026-11-15 | #728 `chore/dng-followup-probes` | ACTIVE | keep | Current detailed evidence or summary table retained by the protected resume pack | docs/reference/decoders/perf/dng-remaining-gap.md:397 |
| `docs/reference/decoders/perf/dng-gap-profiles/first-touch-corrected.txt` | 3548 | 2026-11-15 | #727 `chore/dng-first-touch-probe` | ACTIVE | keep | Current detailed evidence or summary table retained by the protected resume pack | docs/reference/decoders/perf/dng-remaining-gap.md:511 |
| `docs/reference/decoders/perf/dng-gap-profiles/flashraw-counters.txt` | 900 | 2026-11-15 | #727 `chore/dng-first-touch-probe` | ACTIVE | keep | Current detailed evidence or summary table retained by the protected resume pack | docs/reference/decoders/perf/dng-remaining-gap.md:513 |
| `docs/reference/decoders/perf/dng-gap-profiles/flashraw-cpu-followup.txt` | 10849 | 2026-11-16 | #739 `chore/flashraw-cpu-followup` | ACTIVE | keep | Current detailed evidence or summary table retained by the protected resume pack | docs/plans/chore-flashraw-cpu-followup.md:81<br>docs/reference/decoders/perf/dng-gap-profiles/README.md:74<br>docs/reference/decoders/perf/dng-remaining-gap.md:639<br>docs/reference/decoders/perf/flashraw-cpu-comparison.md:105 |
| `docs/reference/decoders/perf/dng-gap-profiles/float-hugepage-retry.txt` | 2573 | 2026-11-15 | #730 `chore/ljpeg-fault-followup` | ACTIVE | keep | Current detailed evidence or summary table retained by the protected resume pack | docs/reference/decoders/perf/dng-remaining-gap.md:409<br>docs/reference/decoders/perf/dng-remaining-gap.md:522<br>docs/reference/decoders/perf/dng-remaining-gap.md:658 |
| `docs/reference/decoders/perf/dng-gap-profiles/large-page-allocation.txt` | 5221 | 2026-11-15 | #738 `chore/large-page-allocation` | ACTIVE | keep | Current detailed evidence or summary table retained by the protected resume pack | docs/reference/decoders/perf/dng-gap-profiles/README.md:61<br>docs/reference/decoders/perf/dng-remaining-gap.md:516<br>docs/reference/decoders/perf/dng-remaining-gap.md:660<br>docs/reference/decoders/perf/dng-remaining-gap.md:83 |
| `docs/reference/decoders/perf/dng-gap-profiles/ljpeg-allocation-followup.txt` | 1428 | 2026-11-15 | #728 `chore/dng-followup-probes` | ACTIVE | keep | Current detailed evidence or summary table retained by the protected resume pack | docs/reference/decoders/perf/dng-remaining-gap.md:258 |
| `docs/reference/decoders/perf/dng-gap-profiles/ljpeg-instruction-followup.txt` | 1144 | 2026-11-15 | #730 `chore/ljpeg-fault-followup` | ACTIVE | keep | Current detailed evidence or summary table retained by the protected resume pack | docs/reference/decoders/perf/dng-remaining-gap.md:296<br>docs/reference/decoders/perf/dng-remaining-gap.md:659 |
| `docs/reference/decoders/perf/dng-gap-profiles/ljpeg-post-c.txt` | 1256 | 2026-11-15 | #727 `chore/dng-first-touch-probe` | ACTIVE | keep | Current detailed evidence or summary table retained by the protected resume pack | docs/reference/decoders/perf/dng-remaining-gap.md:286 |
| `docs/reference/decoders/perf/dng-gap-profiles/ljpeg-predictor-followup.txt` | 1435 | 2026-11-15 | #728 `chore/dng-followup-probes` | ACTIVE | keep | Current detailed evidence or summary table retained by the protected resume pack | docs/reference/decoders/perf/dng-remaining-gap.md:294 |
| `docs/reference/decoders/perf/dng-gap-profiles/ljpeg-remaining-faults.txt` | 2992 | 2026-11-15 | #730 `chore/ljpeg-fault-followup` | ACTIVE | keep | Current detailed evidence or summary table retained by the protected resume pack | docs/reference/decoders/perf/dng-remaining-gap.md:267<br>docs/reference/decoders/perf/dng-remaining-gap.md:521<br>docs/reference/decoders/perf/dng-remaining-gap.md:657 |
| `docs/reference/decoders/perf/dng-gap-profiles/ljpeg-reused-output.txt` | 2613 | 2026-11-15 | #734 `chore/portable-scalar-codec` | ACTIVE | keep | Current detailed evidence or summary table retained by the protected resume pack | docs/reference/decoders/perf/dng-remaining-gap.md:438 |
| `docs/reference/decoders/perf/dng-gap-profiles/ljpeg-table-experiments.txt` | 4089 | 2026-11-15 | #734 `chore/portable-scalar-codec` | ACTIVE | keep | Current detailed evidence or summary table retained by the protected resume pack | docs/reference/decoders/perf/dng-remaining-gap.md:437 |
| `docs/reference/decoders/perf/dng-gap-profiles/ljpeg-unstuff-retry.txt` | 1305 | 2026-11-15 | #728 `chore/dng-followup-probes` | ACTIVE | keep | Current detailed evidence or summary table retained by the protected resume pack | docs/reference/decoders/perf/dng-remaining-gap.md:277 |
| `docs/reference/decoders/perf/dng-gap-profiles/ljpeg.txt` | 3583 | 2026-11-15 | #724 `chore/close-dng-flashraw-gap` | ACTIVE | keep | Current detailed evidence or summary table retained by the protected resume pack | docs/reference/decoders/perf/dng-remaining-gap.md:653 |
| `docs/reference/decoders/perf/dng-gap-profiles/perf-stat-final-ljpeg.txt` | 871 | 2026-11-15 | #724 `chore/close-dng-flashraw-gap` | GENERATED/RAW | archive | Regenerable raw/profile artifact retained under the protected resume policy | - |
| `docs/reference/decoders/perf/dng-gap-profiles/perf-stat.txt` | 1607 | 2026-11-15 | #724 `chore/close-dng-flashraw-gap` | GENERATED/RAW | archive | Regenerable raw/profile artifact retained under the protected resume policy | - |
| `docs/reference/decoders/perf/dng-gap-profiles/pixel8-ljpeg-flashraw.folded` | 15885 | 2026-11-15 | #724 `chore/close-dng-flashraw-gap` | GENERATED/RAW | archive | Regenerable raw/profile artifact retained under the protected resume policy | - |
| `docs/reference/decoders/perf/dng-gap-profiles/pixel8-ljpeg-flashraw.svg` | 9435 | 2026-11-15 | #724 `chore/close-dng-flashraw-gap` | GENERATED/RAW | archive | Regenerable raw/profile artifact retained under the protected resume policy | - |
| `docs/reference/decoders/perf/dng-gap-profiles/pixel8-ljpeg-fornax.folded` | 114666 | 2026-11-15 | #724 `chore/close-dng-flashraw-gap` | GENERATED/RAW | archive | Regenerable raw/profile artifact retained under the protected resume policy | - |
| `docs/reference/decoders/perf/dng-gap-profiles/pixel8-ljpeg-fornax.svg` | 30821 | 2026-11-15 | #724 `chore/close-dng-flashraw-gap` | GENERATED/RAW | archive | Regenerable raw/profile artifact retained under the protected resume policy | - |
| `docs/reference/decoders/perf/dng-gap-profiles/portable-allocation-regression.txt` | 4921 | 2026-11-15 | #734 `chore/portable-scalar-codec` | ACTIVE | keep | Current detailed evidence or summary table retained by the protected resume pack | docs/reference/decoders/perf/dng-remaining-gap.md:154<br>docs/reference/decoders/perf/dng-remaining-gap.md:419<br>docs/reference/decoders/perf/dng-remaining-gap.md:458<br>docs/reference/decoders/perf/dng-remaining-gap.md:518 |
| `docs/reference/decoders/perf/dng-gap-profiles/portable-scalar.txt` | 6948 | 2026-11-15 | #734 `chore/portable-scalar-codec` | ACTIVE | keep | Current detailed evidence or summary table retained by the protected resume pack | docs/reference/decoders/perf/dng-remaining-gap.md:436 |
| `docs/reference/decoders/perf/dng-gap-profiles/profiles.txt` | 1795 | 2026-11-15 | #724 `chore/close-dng-flashraw-gap` | ACTIVE | keep | Current detailed evidence or summary table retained by the protected resume pack | docs/reference/decoders/perf/dng-remaining-gap.md:200<br>docs/reference/decoders/perf/dng-remaining-gap.md:228<br>docs/reference/decoders/perf/dng-remaining-gap.md:652 |
| `docs/reference/decoders/perf/dng-gap-profiles/same-session-ratios.txt` | 2058 | 2026-11-15 | #730 `chore/ljpeg-fault-followup` | ACTIVE | keep | Current detailed evidence or summary table retained by the protected resume pack | docs/reference/decoders/perf/dng-remaining-gap.md:472<br>docs/reference/decoders/perf/dng-remaining-gap.md:520<br>docs/reference/decoders/perf/dng-remaining-gap.md:655 |
| `docs/reference/decoders/perf/dng-gap-profiles/synthetic-ljpeg.csv` | 448 | 2026-11-15 | #724 `chore/close-dng-flashraw-gap` | GENERATED/RAW | archive | Regenerable raw/profile artifact retained under the protected resume policy | - |
| `docs/reference/decoders/perf/dng-gap-profiles/thread-scaling.txt` | 1050 | 2026-11-15 | #727 `chore/dng-first-touch-probe` | ACTIVE | keep | Current detailed evidence or summary table retained by the protected resume pack | - |
| `docs/reference/decoders/perf/dng-gap-profiles/timings.txt` | 1768 | 2026-11-15 | #724 `chore/close-dng-flashraw-gap` | ACTIVE | keep | Current detailed evidence or summary table retained by the protected resume pack | - |
| `docs/reference/decoders/perf/dng-gap-profiles/uncompressed.txt` | 2414 | 2026-11-15 | #724 `chore/close-dng-flashraw-gap` | ACTIVE | keep | Current detailed evidence or summary table retained by the protected resume pack | docs/reference/decoders/perf/dng-remaining-gap.md:654 |
| `docs/reference/decoders/reference/dng-lossless-jpeg.md` | 5148 | 2026-11-14 | #720 `task/perf-dng-decode` | ACTIVE | keep | Current architecture, decoder, reference, or active guidance | docs/README.md:76<br>docs/decoder/formats/dng.md:154<br>docs/reference/decoders/dng-performance.md:24 |
| `docs/reference/decoders/reference/dng-opcodes.md` | 2883 | 2026-11-12 | #700 | ACTIVE | keep | Current architecture, decoder, reference, or active guidance | docs/README.md:77<br>docs/reference/decoders/reference/dng-lossless-jpeg.md:56 |
| `docs/reference/decoders/reference/dng-output-color.md` | 2703 | 2026-11-12 | #653 | ACTIVE | keep | Current architecture, decoder, reference, or active guidance | docs/README.md:83 |
| `docs/reference/decoders/dng-performance.md` | 15389 | 2026-11-14 | #720 `task/perf-dng-decode` | SUPERSEDED | archive | Replaced by the current gap report and consolidated experiment log | docs/README.md:78<br>docs/plans/perf-dng-decode.md:27<br>docs/reference/decoders/perf/dng-remaining-gap.md:650 |
| `docs/reference/decoders/perf/dng-remaining-gap.md` | 37766 | 2026-11-16 | #720 `task/perf-dng-decode` | ACTIVE | keep + trim | Single current DNG report; move history to the experiment log and remove branch-era wording | docs/README.md:80<br>docs/plans/chore-close-dng-flashraw-gap.md:26<br>docs/plans/perf-dng-decode.md:1253<br>docs/reference/decoders/perf/dng-experiments-log.md:4<br>docs/reference/decoders/perf/dng-gap-profiles/README.md:53<br>docs/reference/decoders/perf/dng-gap-profiles/RESUME.md:5<br>docs/reference/decoders/dng-performance.md:8 |
| `docs/reference/decoders/perf/flashraw-cpu-comparison.md` | 10498 | 2026-11-16 | #738 `chore/large-page-allocation` | ACTIVE | keep | Current architecture, decoder, reference, or active guidance | docs/plans/chore-flashraw-cpu-followup.md:20<br>docs/plans/chore-flashraw-cpu-followup.md:53<br>docs/reference/decoders/perf/dng-gap-profiles/README.md:71<br>docs/reference/decoders/perf/dng-gap-profiles/RESUME.md:7 |
| `docs/reference/decoders/reference/flashraw.md` | 2820 | 2026-11-12 | #717 | ACTIVE | keep | Current architecture, decoder, reference, or active guidance | docs/README.md:84<br>docs/decoder/benchmark.md:147<br>docs/reference/decoders/dng-performance.md:18 |
| `docs/reference/decoders/reference/fuji-raf.md` | 1667 | 2026-11-12 | #708 | ACTIVE | keep | Current architecture, decoder, reference, or active guidance | crates/fornax-codec/README.md:104<br>crates/fornax-raw-fujifilm/README.md:22<br>docs/README.md:85 |
| `docs/reference/decoders/reference/hasselblad-3fr.md` | 2197 | 2026-11-12 | #710 | ACTIVE | keep | Current architecture, decoder, reference, or active guidance | docs/README.md:86<br>docs/decoder/formats/hasselblad-3fr.md:91 |
| `docs/reference/decoders/reference/nikon-nef.md` | 12134 | 2026-11-12 | #705 | ACTIVE | keep | Current architecture, decoder, reference, or active guidance | docs/README.md:87 |
| `docs/reference/decoders/reference/panasonic-rw2.md` | 3543 | 2026-11-12 | #712 | ACTIVE | keep | Current architecture, decoder, reference, or active guidance | crates/fornax-codec/README.md:108<br>crates/fornax-raw-panasonic/README.md:30<br>docs/README.md:88<br>docs/architecture/decoder.md:252 |
| `docs/reference/decoders/reference/phase-one-iiq.md` | 3529 | 2026-11-12 | #713 | ACTIVE | keep | Current architecture, decoder, reference, or active guidance | crates/fornax-codec/README.md:106<br>crates/fornax-raw-phase-one/README.md:18<br>docs/README.md:89<br>docs/decoder/formats/phase-one-iiq.md:96 |
| `docs/reference/decoders/design/restart-free-single-scan-parallelism-design.md` | 4944 | 2026-11-16 | #739 `chore/flashraw-cpu-followup` | ACTIVE | keep | Current architecture, decoder, reference, or active guidance | docs/reference/decoders/perf/dng-gap-profiles/README.md:77<br>docs/reference/decoders/perf/dng-gap-profiles/RESUME.md:8<br>docs/reference/decoders/perf/dng-remaining-gap.md:637<br>docs/reference/decoders/perf/flashraw-cpu-comparison.md:102 |
| `docs/reference/decoders/reference/sony-arw.md` | 5609 | 2026-11-12 | #678 | ACTIVE | keep | Current architecture, decoder, reference, or active guidance | docs/README.md:90<br>docs/decoder/formats/sony-arw.md:108<br>docs/reference/codecs/compression-ids.md:58<br>docs/reference/decoders/reference/dng-compression.md:14 |

## Experiment Branches and Worktrees

All entries are recommendations only. Do not delete branches or worktrees.

| Branch or detached worktree | Location | Status | Recommendation |
| --- | --- | --- | --- |
| `task/perf-dng-decode` | `.worktrees/perf-dng-decode` (prunable metadata) | Merged experiment line | Keep |
| `chore/close-dng-flashraw-gap` | `.worktrees/chore-close-dng-flashraw-gap` | Merged | Keep |
| `chore/dng-first-touch-probe` | `.worktrees/chore-dng-first-touch-probe` | Merged | Keep |
| `chore/dng-followup-probes` | `.worktrees/chore-dng-followup-probes` | Merged | Keep |
| `chore/large-page-allocation` | `.worktrees/chore-large-page-allocation` | Merged | Keep |
| detached `985c704` | `.worktrees/chore-large-page-allocation-baseline` | Baseline | Keep |
| `chore/ljpeg-critical-path` | `.worktrees/chore-ljpeg-critical-path` | Merged | Keep |
| `chore/ljpeg-fault-followup` | `.worktrees/chore-ljpeg-fault-followup` | Merged | Keep |
| `chore/ljpeg-instruction-gap` | `.worktrees/chore-ljpeg-instruction-gap` | Merged | Keep |
| `chore/portable-ljpeg-fused-kernel` | `.worktrees/chore-portable-ljpeg-fused-kernel` | Negative result | Keep |
| `chore/portable-scalar-codec` | `.worktrees/chore-portable-scalar-codec` | Merged | Keep |
| `chore/flashraw-cpu-comparison` | `.worktrees/chore-flashraw-cpu-comparison` | Comparison branch | Keep |
| `chore/flashraw-cpu-followup` | `.worktrees/chore-flashraw-cpu-followup` | Merged | Keep |
| `chore/flashraw-cpu-source` | no active worktree | Negative/source branch | Keep |
| `chore/arw-gap-profiles` | `.worktrees/chore-arw-gap-profiles` | Merged | Keep |
| detached LJPEG baseline | `/tmp/fornax-ljpeg-baseline` (prunable metadata) | Baseline metadata only | Keep |

## Task Checklist

- [x] Inventory all current docs paths, sizes, last-commit dates, and
  introducing PR/branch evidence.
- [x] Collect inbound links for every docs path.
- [x] Create the protected DNG performance resume pack.
- [x] Record the cleanup proposal.
- [x] Obtain review approval for this table.
- [x] Apply the approved archive/index/trim actions.
- [x] Update every inbound link and keep AGENTS.md pointers valid.
- [x] Add the CI-runnable link check if no equivalent exists.
- [x] Run docs-only validation and report file-count/size changes.
- [x] Commit, push, and update the docs-only PR.

## Files Expected to Change

- The paths listed with a non-`keep` action in the inventory.
- `docs/README.md`, `docs/CONVENTIONS.md`, and `docs/archive/README.md`.
- Inbound-link owners: `docs/architecture/overview.md`,
  `crates/fornax-raw-py/README.md`, and any profile-index links affected by
  generated-artifact moves.
- `scripts/` and `.github/workflows/` only if the repository has no suitable
  link check and the approved implementation needs a small checker.

## Validation

- `git diff --check`
- Repository Markdown/link check, including source comments and README links
  into `docs/reference/decoders/perf/dng-gap-profiles/`.
- `scripts/agent-validation.sh --docs-only`
- Requested binary-extension scan:
  `git ls-files docs | grep -E '\.(png|jpe?g|gif|pdf|zip|bin)$'`
- Before/after `du -sh docs` and tracked file counts.
- Confirm all branch and worktree experiment sources remain present.

## Decisions and Notes

- The archive-root rename is a structural compliance decision, not a content
  rewrite. Relative links inside the archive tree keep working; external
  inbound links are updated.
- Generated `.folded`, `.svg`, `perf-stat`, and synthetic CSV files are
  archived, not deleted, because the resume pack forbids deletion and requires
  preservation when regeneration cannot be guaranteed.
- `dng-remaining-gap.md` remains the only current DNG performance report.
  Step-by-step history goes to `dng-experiments-log.md` and the profile archive.
- `docs/plans/book-03-pdf-build.md` and
  `docs/plans/compare-demosaic-algorithms.md` remain active.
- No branch or worktree is deleted.
