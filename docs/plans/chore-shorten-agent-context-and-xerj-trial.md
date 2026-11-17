# Shorten agent context and trial XERJ Implementation Plan

> **For agentic workers:** Execute this plan inline. Keep codec production code
> unchanged. Use checkbox steps to track progress.

**Goal:** Reduce Fornax's always-loaded project context to at most 40% of its
current size without losing rules, then measure XERJ on one NEF design task.

**Architecture:** Keep only universal workflow, validation, safety, and minimal
repository-shape rules in root `AGENTS.md`. Move task-specific procedure into
`docs/agent-workflow.md`, retain existing architecture/reference docs as
on-demand sources, and add a generic output-capture wrapper only if a small
script can improve quiet tooling safely. The XERJ trial is analysis-only and
produces a bounded report under `docs/plans/`.

**Tech Stack:** Markdown, Bash, Git, Rust workspace commands, XERJ.

## Global Constraints

- No RAW codec or performance implementation work beyond the one design note.
- Task A changes documentation and agent-context files; A4 may change a helper
  script only.
- Keep `AGENTS.md` agent-tool-agnostic; no `CLAUDE.md`; project skills remain in
  `.agents/skills/`; `TODO.md` remains human-owned.
- Commit no binary files.
- XERJ stays on localhost, uses its existing configuration and data directory,
  and is not reinstalled or reconfigured.
- Do not open a XERJ field-report pull request or run `xerj feedback --open-pr`.
- Do not copy source from restricted-licence reference repositories.
- Record each indexed reference repository's licence.
- Run Run 1 without XERJ and Run 2 with XERJ; do not reuse Run 1 notes in Run 2.
- The final report stays under 100 lines and is not linked from always-loaded
  context.

## Scope

In scope:

- Inventory and classify session-start and explicitly referenced context.
- Rewrite root `AGENTS.md`; add on-demand workflow/audit docs.
- Add a small quiet-command helper if validation proves it safe.
- Inspect, index, and query only the requested codec/vendor sources.
- Produce the NEF Huffman design comparison, A/B measurements, and verdict.

Out of scope:

- NEF or other codec implementation.
- Benchmark or performance optimization.
- XERJ feedback/field-report workflow.
- Rewriting nested package guides or human-owned `TODO.md`.

## Inventory Baseline

Byte counts are from the branch base. Token estimates use bytes / 4.

| File | Bytes | Est. tokens | Loading class |
| --- | ---: | ---: | --- |
| `AGENTS.md` | 10,996 | 2,749 | unconditional |
| `/home/glatzel/.pixi/envs/ai/RTK.md` | 547 | 137 | unconditional, externally managed |
| `.agents/skills/decoder-corpus-tracker/SKILL.md` description | 216 | 54 | unconditional catalog description |
| `.agents/skills/demosaic-comparison/SKILL.md` description | 327 | 82 | unconditional catalog description |
| **Always-loaded total** | **12,086** | **3,022** | target is 4,834 bytes |
| `TODO.md` | 2,207 | 552 | conditional on task-ID selection |
| `docs/architecture/overview.md` | 19,285 | 4,821 | conditional on architecture work |
| `crates/compare-demosaic/AGENTS.md` | 7,549 | 1,887 | conditional nearest-file guide |
| `book/AGENTS.md` | 4,530 | 1,133 | conditional nearest-file guide |
| `crates/decoder-tracker/AGENTS.md` | 8,690 | 2,173 | conditional nearest-file guide |
| Project skill bodies | 14,563 | 3,641 | conditional when skill invoked |

The all-files inventory sum is 68,367 bytes. It is not a session total because
the conditional files are not all loaded together.

## Task Checklist

### Task 1: Rewrite always-loaded context

- [x] Classify every root rule as must-know, sometimes, or stale/redundant.
- [x] Replace eager architecture/TODO reads with task-triggered pointers.
- [x] Write detailed workflow, validation, PR, and reference procedure to
      `docs/agent-workflow.md`.
- [x] Record the old-rule-to-new-location audit in
      `docs/plans/chore-agent-context-audit.md`.
- [x] Keep root `AGENTS.md` at or below 3,744 bytes so the total remains at or
      below 40%.
- [x] Run `scripts/agent-validation.sh --docs-only`.
- [x] Commit the context rewrite and moves as one docs commit.

### Task 2: Quiet noisy agent tooling

- [x] Inspect validation, tracker, comparison, benchmark, and CI helpers.
- [x] Add a small generic output-capture helper only if it preserves exit status,
      full logs, and useful diagnostics.
- [x] Test success, failure, and log-location behavior.
- [x] Document only the command line needed in always-loaded context; put usage
      detail in the on-demand workflow doc.
- [x] Commit tooling separately or report why no safe change was made.

### Task 2b: Consolidate duplicated decoder documentation

- [x] Compare `docs/decoder/formats/` with
      `docs/reference/decoders/reference/`.
- [x] Make format pages compact support/acceptance summaries with stable URLs.
- [x] Make decoder reference notes the canonical detailed source/behavior
      authority, adding a canonical DNG decoder note.
- [x] Update conventions and documentation indexes without changing production
      code.

### Task 3: XERJ setup and measurements

- [ ] Confirm the node is local and running without changing configuration.
- [ ] Index only the requested Fornax codec/vendor crates, FlashRaw, and LibRaw
      when present.
- [ ] Record file counts, record counts, elapsed time, paths, commits, and
      licences.
- [ ] Execute Run 1 with normal file tools only.
- [ ] Execute Run 2 in a separate context using XERJ retrieval.
- [ ] Run `xerj gain` after Run 2.

### Task 4: Design comparison and verdict

- [ ] Write the short NEF Huffman design note from each run.
- [ ] Verify at least three claims from each note against source `file:line`.
- [ ] Compare coverage and correctness, then calculate measured deltas.
- [ ] Write `docs/plans/xerj-trial-2026-11.md` under 100 lines.
- [ ] Run documentation validation and commit the report.

## Files Expected to Change

- `AGENTS.md`
- `docs/agent-workflow.md`
- `docs/plans/chore-agent-context-audit.md`
- `docs/plans/chore-shorten-agent-context-and-xerj-trial.md`
- `docs/decoder/formats/` support summaries and
  `docs/reference/decoders/reference/` canonical notes
- `scripts/agent-run.sh` and a focused test script, if the helper is adopted
- `docs/plans/xerj-trial-2026-11.md`

No production Rust source, benchmark, algorithm, baseline, or corpus file is
expected to change.

## Validation

- Always-loaded byte count command compares the new root bundle with 12,086.
- `scripts/agent-validation.sh --docs-only` checks links and diff whitespace.
- `bash scripts/agent-run-test.sh` proves the quiet helper if added.
- `xerj gain` records the tool's own savings figure after Run 2.
- Design-note claims are checked at source `file:line`; the report lists them.

## Decisions and Notes

- The measured always-loaded set excludes global Codex skill catalogs because
  Fornax cannot shorten externally managed installation content. It includes
  the RTK pointer because the harness imports it and project skill descriptions
  because the harness advertises them.
- `TODO.md` and architecture remain canonical documents. They become
  on-demand reads rather than duplicated session context.
- The existing XERJ note in `AGENTS.md` is already a local pointer rule; the
  trial may replace it with one equally short adoption line only if evidence
  supports adoption.
- Task A result after the quiet-tooling line: root `AGENTS.md` is 3,373 bytes;
  the unconditional bundle is 4,463 bytes (36.9% of the 12,086-byte baseline,
  about 1,116 estimated tokens).
- Decoder format/reference consolidation reduces the two active trees from
  89,837 to 71,767 bytes by keeping detailed behavior in one place and retaining
  stable support-page URLs.
