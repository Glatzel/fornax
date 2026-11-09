# Fornax workspace refactor proposal

**Status: PROPOSAL, not implemented.**

## Baseline

| Item | Value | Evidence |
| --- | --- | --- |
| Commit | `9b9c1b284ff5f4fe9ef1c8ff41eb6c3cff17b8d2` | `[CURRENT]` `git rev-parse HEAD` |
| Branch | `main` | `[CURRENT]` `git branch --show-current` |
| Initial worktree state | `M external/raw.pixls.us` | `[CURRENT]` `git status --short`; this was pre-existing |
| Proposal date | `2026-11-09T18:50:13+08:00` | `[CURRENT]` `date -Iseconds` |
| Earlier export commit | `9b9c1b284ff5f4fe9ef1c8ff41eb6c3cff17b8d2` | `[CURRENT]` `target/architecture-export/architecture-export.md:7` |
| Earlier export relationship | Same commit as this proposal | `[CURRENT]` `git rev-parse HEAD` |

`[CURRENT]` The earlier export remains usable as a subject index because it was
generated at the same commit, but all reused facts in these documents were
re-checked against source with `rg` and numbered file reads.

`[CURRENT]` Re-verification corrected one export conclusion: the export's
raw-term matrix says `fornax-codec` has no raw-workflow terms, yet the crate
imports `RawMosaic` at `crates/fornax-codec/src/lib.rs:49` and returns it from
`decode` at `crates/fornax-codec/src/lib.rs:90`.

`[CURRENT]` The earlier export's 13-member inventory, workspace dependency
graph, feature inventory, type ownership, and stale-document findings were
spot-checked with `cargo metadata --format-version 1 --no-deps`,
`cargo tree --workspace`, `rg`, and numbered file reads, and retained where the
source still agreed; this proposal adds more direct citations and the target
mapping.

## Reading the tags

`[CURRENT]` means the statement is true at the baseline and is followed by a
repo-relative `path:line` citation or by a command that reproduces the fact.

`[TARGET]` means the statement describes the maintainer-selected proposal and
is not implemented.

`[TARGET]` The documents leave every item marked OPEN undecided; options and
evidence are recorded in [06-open-decisions.md](06-open-decisions.md).

## Document index

| Document | Content |
| --- | --- |
| [01-current-architecture.md](01-current-architecture.md) | `[CURRENT]` crate graph, flow, and evidence-backed pain points |
| [02-target-architecture.md](02-target-architecture.md) | `[TARGET]` workspaces, dependency rules, features, and API sketches |
| [03-mapping.md](03-mapping.md) | `[TARGET]` current-to-target crate, file, type, API, constant, and helper mapping |
| [04-migration-plan.md](04-migration-plan.md) | `[TARGET]` ordered migration slices and risk register |
| [05-docs-impact.md](05-docs-impact.md) | `[TARGET]` impact on every existing Markdown file |
| [06-open-decisions.md](06-open-decisions.md) | OPEN choices and evidence gaps |
| [readmes/](readmes/) | `[TARGET]` draft README for each target crate |
| [sanitize-checklist.md](sanitize-checklist.md) | Locations to review before sharing |
| [claude-summary.md](claude-summary.md) | Self-contained feasibility-analysis brief |

## Evidence conventions

`[CURRENT]` LOC means non-blank lines under `src/**/*.rs`, reproduced with
`find crates -path '*/src/*.rs' -type f -print0 | xargs -0 awk 'NF{n++} END{print n}'`.

`[CURRENT]` Test counts mean lines carrying `#[test]` or `#[rstest]` in source,
integration-test, and benchmark trees, reproduced with
`rg -n '#\[(tokio::)?test\]|#\[rstest\]' crates -g '*.rs' | wc -l`.

`[TARGET]` Sizes in the migration plan are explicitly estimates; they are not
measurements.

`[TARGET]` No file in this directory is intended to be merged as live project
documentation before the maintainer decides on the refactor.
