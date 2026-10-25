## Agent task

- Task ID:
- Task manifest:
- Branch and worktree:
- Integration base: `refactor/raw-pipeline-v2`

## Scope

Describe the requested outcome and intentionally excluded work.

## Decisions

Summarize API, numerical, decoder, and workflow decisions that need review.

## Validation

List each command and paste the actual result. State skipped checks explicitly.

```text
cargo test -p fornax --lib
cargo test -p fornax --features backend-libraw --lib
cargo test -p fornax --features backend-libraw --examples
cargo fmt --check
cargo clippy -p fornax --all-targets --features backend-libraw -- -D warnings
python3 agent-work/tasks/validate.py
```

## Risks and follow-up

Record limitations and deferred work. White balance, color correction, and
future postprocessing remain explicitly undecided.

## Reviewer checklist

- [ ] The task manifest matches the branch and PR.
- [ ] The branch starts from and targets `refactor/raw-pipeline-v2`.
- [ ] Validation results are complete and reproducible.
- [ ] The branch contains only this task's changes.
- [ ] `STATUS.md` is marked `Done` after the PR opens.
- [ ] The agent has not merged, closed, watched, or polled the PR.
