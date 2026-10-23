# Agent Instructions — fornax

Detailed agent, research, and design instructions live in
[`agent-work/AGENTS.md`](agent-work/AGENTS.md). The authoritative RAW-pipeline
workflow is [`agent-work/raw-pipeline.md`](agent-work/raw-pipeline.md), and live
task status is tracked in
[`agent-work/tasks/raw-pipeline.md`](agent-work/tasks/raw-pipeline.md). Read
those files before changing the RAW pipeline.

Keep agent-maintained materials under `agent-work/`; do not write to the
read-only `.agents/` directory or add reference repositories to production
builds. Preserve the pipeline boundary, structured metadata semantics, and the
task gates defined by the authoritative workflow.
