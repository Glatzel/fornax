# Agent Instructions — fornax

`TODO.md` is the authoritative backlog and completion record. Read it before
selecting implementation work. Agent instructions live in this root file and
in no other directory.

Until `refactor/pipeline-v3` is merged into `main`, use
`refactor/pipeline-v3` as the integration base. Once it has merged, use `main`
as the integration base for every new task and bug-fix branch.

## Task work

The user selects work by its bracketed ID in `TODO.md`, for example `[VNG]`.
Each selected TODO item maps to exactly one branch, one worktree, and one pull
request:

1. Fetch `origin` and create a branch and worktree from the current integration
   base:

   ```bash
   git fetch origin
   git worktree add -b "task/<lowercase-id>-<slug>" \
     ".worktrees/<lowercase-id>-<slug>" origin/<integration-base>
   ```

   Use a short lowercase kebab-case slug derived from the item and substitute
   the current integration base from the rule above.
2. Implement only the selected TODO item and its required tests or
   documentation.
3. Remove that item from `TODO.md` in the same branch and pull request. The
   removal means the task is complete in that pull request.
4. Run the relevant validation commands and record the actual results.
5. Commit, push, and open one semantic pull request targeting the current
   integration base. Include the task ID in the pull request body.
6. Report the pull request URL and stop. Do not watch, poll, merge, or close
   the pull request.

Keep the TODO item when work is incomplete or blocked. Do not mark completion
in another file. Do not recreate per-task manifests, `STATUS.md`,
`agent-work/AGENTS.md`, design gates, or checkpoint trees; those are obsolete
workflow artifacts.

## Bug work

The user reports bugs through GitHub issues and supplies the issue number.
Handle only the named issue:

1. Read the issue with `gh issue view <number>`.
2. Fetch `origin` and create a branch and worktree for that issue:

   ```bash
   git fetch origin
   git worktree add -b "fix/<number>-<slug>" \
     ".worktrees/fix-<number>-<slug>" origin/<integration-base>
   ```

3. Implement the fix, add regression coverage, and validate the affected
   surface.
4. Open one semantic pull request targeting the current integration base and
   include `Fixes #<number>` in its body.
5. Report the pull request URL and stop. Do not watch, poll, merge, manually
   close the issue, or close the pull request.

A bug-fix pull request addresses one issue. Do not change `TODO.md` for issue
work unless the user explicitly identifies a TODO item in the request.

## Shared workspace

`agent-work/` is a shared workspace for temporary or persistent files that
agents need to exchange between tasks. It is intentionally not ignored. Files
may be committed there when they are needed by multiple tasks, but they are
not task state, task instructions, or completion records.

The hidden `.agents/` directory is read-only. Do not add files to it.

## Project boundaries

Fornax is a Rust workspace using edition 2024. Keep decoding, preprocessing,
and demosaicing as separate stages. The active demosaic target is conventional
2x2 RGB Bayer input; do not silently claim support for X-Trans, four-color,
monochrome, or other CFA layouts.

Do not copy or reintroduce the `refactor/raw-pipeline-v2` research, design,
task-manifest, status, or checkpoint files. Refer to that branch only when
directed by the task.

## Validation

For Rust changes, run the applicable commands from the repository root:

```bash
cargo test --workspace --all-features
cargo fmt --check
cargo clippy --workspace --all-features --all-targets -- -D warnings
```

Run relevant examples, benchmarks, or task-specific tests as well. For
documentation-only changes, run `git diff --check` and any relevant Markdown
checks. Report failures and skipped checks explicitly; do not describe an
unverified check as passing.

## Pull request handoff

Use a title accepted by `.github/semantic.yml`. If the user requests changes,
update the same branch and pull request. Only the user merges or abandons a
pull request; the agent stops after reporting it.
