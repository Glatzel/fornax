# Progress Checkpoints

Use this directory for durable pause, interruption, and handoff checkpoints.
The task manifest at `agent-work/tasks/raw-pipeline.md` remains the authority
for task status; a checkpoint records details needed to resume work.

Name checkpoints by task and date, for example
`2026-10-23-RES-03.md`. Include:

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
