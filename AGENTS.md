# Agent Instructions — fornax

Read [`agent-work/AGENTS.md`](agent-work/AGENTS.md) and the authoritative
[`agent-work/workflows/raw-pipeline.md`](agent-work/workflows/raw-pipeline.md)
before changing the RAW pipeline.

Keep agent-maintained material under `agent-work/`; never write to `.agents/`.
Each task uses one manifest, one `task/<id>-<slug>` branch, and one
`.worktrees/<id>-<slug>` worktree. Unfinished implementation work starts from
and targets `origin/refactor/raw-pipeline-v2`.

Only the user may merge a task pull request or promote the integration branch
to `main`.
