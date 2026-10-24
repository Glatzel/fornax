# Workflow Restructure Checkpoint — 2026-10-23

> Historical workflow checkpoint. Task status and review approval now live in
> GitHub pull requests; status wording below is preserved as history.

**Scope:** Organizational restructuring only. No new research findings, API
design, or implementation were added during this work.

## Preserved task state

- `RES-01` and `RES-02` remain in `review`; their existing reports were not
  restarted or reworked.
- `RES-03`–`RES-06` remain `in_progress` as recorded in the authoritative task
  manifest before restructuring.
- `SYN-01`–`SYN-03`, all design tasks, and all implementation tasks remain
  blocked by their recorded dependencies.

## Workspace changes

- Detailed agent instructions moved from the root `AGENTS.md` into
  `agent-work/AGENTS.md`; the root file now points to the detailed instructions
  and authoritative workflow.
- Research checkouts were moved from root-level `.research-cache-*` directories
  to `agent-work/references/{darktable,dnglab,other/rawloader}`.
- Reference provenance and exact revisions are recorded in
  `agent-work/references/README.md`.
- The production `external/raw-images` fixture submodule remains in place because
  existing tests and development tools consume it.
- The empty design marker was initially replaced by an
  `agent-work/design/raw-pipeline.md` placeholder. That placeholder is retired;
  the persistent design now lives under repository-root `docs/raw-pipeline/`.

## Resume point

Resume each active task from its current report, task entry, and checkpoint.
Verify the reference revisions in `agent-work/references/README.md` first.
Continue research only when the user requests it; do not infer that this
organizational pass completed any research acceptance criterion.
