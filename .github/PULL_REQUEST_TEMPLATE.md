## Agent task

- Task ID:
- Task manifest:
- Dependencies and merged PRs:
- Branch and worktree:

## Scope

Describe the requested outcome and the files that are intentionally in scope.
List any shared interfaces or cross-task files and explain how their ownership
is serialized.

## Review notes

Point reviewers to the decisions that need judgment. Separate verified facts
from inferences and recommendations when the task produces research evidence.

## Validation

List each command or review check and paste the actual result. State any check
that could not run and why.

At minimum, run `python3 agent-work/tasks/validate.py` when this PR changes a
task manifest, task index, or task-owned deliverable.

## Risks and follow-up

Describe known limitations, deferred work, migration impact, and any follow-up
task that needs its own branch and PR.

## Reviewer checklist

- [ ] The task ID is in the PR title and the linked manifest matches the scope.
- [ ] Every dependency PR is merged.
- [ ] Deliverables use the paths and names in the task manifest.
- [ ] The branch contains only this task's changes.
- [ ] Validation results are complete and reproducible.
- [ ] Review comments are resolved in this PR.
- [ ] The user, not the agent, approves and merges the PR.
