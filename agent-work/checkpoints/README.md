# Progress Checkpoints

Use this directory for durable pause, interruption, and handoff checkpoints.
The task's open pull request is the authority for live status; a checkpoint
records details needed to resume work.

Store task checkpoints under `agent-work/checkpoints/<TASK-ID>/` and name them
`<YYYY-MM-DD>-<slug>.md`, for example
`agent-work/checkpoints/RES-03/2026-10-23-final-evidence.md`. Cross-task
workflow checkpoints live under `agent-work/checkpoints/workflow/`.

Each checkpoint includes:

- the task ID and owner;
- the source revision or fixture set inspected;
- completed findings and where they are recorded;
- unfinished work, next steps, and unresolved questions;
- command results and other validation evidence;
- any blocker or dependency change that affects readiness.

Before resuming, read the authoritative task definition, the task's owned
report or note, and its latest checkpoint. Verify the recorded source revisions
and dependencies, then continue from the checkpoint without repeating verified
work. Do not use a checkpoint to bypass synthesis or design gates.

Do not record review approval or completion in a checkpoint. Leave the task PR
open and continue in the same branch/worktree when resuming.
