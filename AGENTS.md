# Agent Instructions — fornax

Detailed agent, research, and design instructions live in
[`agent-work/AGENTS.md`](agent-work/AGENTS.md). The authoritative RAW-pipeline
workflow is
[`agent-work/workflows/raw-pipeline.md`](agent-work/workflows/raw-pipeline.md),
and task contracts are stored as one TOML manifest per task under
[`agent-work/tasks/raw-pipeline/`](agent-work/tasks/raw-pipeline/). Read those
files before changing the RAW pipeline.

Keep agent-maintained materials under `agent-work/`; do not write to the
read-only `.agents/` directory or add reference repositories to production
builds. Preserve the pipeline boundary, structured metadata semantics, and the
task gates defined by the authoritative workflow. Execute each task in its own
`task/<id>-<slug>` branch and `.worktrees/<id>-<slug>` worktree, and submit it
through the task's GitHub pull request. GitHub pull-request review is the only
approval record.
