# Task Manifests

Each agent task is a single TOML file under
`agent-work/tasks/<workflow>/<TASK-ID>.toml`. TOML is used because task
contracts are structured data: IDs, dependencies, deliverables, acceptance
criteria, and validation can be parsed and checked without scraping prose.
Markdown remains the format for task-owned reports and design proposals.

One file per task is intentional. It gives every branch a narrow, conflict-free
metadata change and lets GitHub review the task contract alongside its output.

TOML is the best fit for these manifests: it is unambiguous, supports comments
and readable multiline strings, works naturally with the repository's Rust and
Pixi tooling, and parses without a service or schema dependency. JSON is harder
to review because it cannot carry explanatory comments. YAML is convenient for
CI configuration but has more implicit type and indentation rules. Markdown is
better for evidence and decisions, so those remain task-owned `.md` files.

Validate all manifests and the dependency graph from the repository root:

```bash
python3 agent-work/tasks/validate.py
```

## Manifest schema

```toml
schema_version = 1
id = "RES-01"
slug = "fornax-inventory"
title = "Inventory existing fornax components"
phase = "research"
dependencies = []
parallelism = "independent with RES-02..RES-08"
deliverables = ["agent-work/research/RES-01-fornax-inventory.md"]
scope = """
What this task covers and explicitly does not cover.
"""
acceptance = [
  "An objectively reviewable acceptance criterion.",
]
validation = [
  "A command, evidence check, or review step to perform.",
]
```

Required fields are `schema_version`, `id`, `slug`, `title`, `phase`,
`dependencies`, `deliverables`, `scope`, `acceptance`, and `validation`.
`parallelism` is optional. `requires_one_of` is an optional array for tasks that
accept any one of several prerequisite implementations; it does not replace
the mandatory `dependencies` array. Files must use the task ID as their basename:
`RES-01.toml`, `DES-007.toml`, and so on.

Do not add live status, review approval, branch URL, or completion fields.
Those facts belong to Git and GitHub and would otherwise become stale:

- dependencies are complete when their task pull requests are merged;
- a task is ready when all dependencies are merged and no task PR is active;
- a task is in progress or review while its task PR is open;
- a task is complete when the user approves and merges its pull request;
- an abandoned task PR is closed and its branch/worktree is removed.

## Branch and worktree contract

For task `RES-01` with slug `fornax-inventory`:

```bash
git fetch origin
git worktree add -b task/res-01-fornax-inventory \
  .worktrees/res-01-fornax-inventory origin/agent
cd .worktrees/res-01-fornax-inventory
```

The default integration base is `origin/agent`. Use another base only when the
task index defines an intentional stacked dependency. The repository ignores
`.worktrees/`; worktrees are execution state and must not be committed.

Use `task/<lowercase-task-id>-<slug>` for the branch and
`.worktrees/<lowercase-task-id>-<slug>` for the worktree. A slug is immutable
once work starts because branches and PR search rely on it.

## Pull-request contract

Each task has exactly one review PR, titled `[TASK-ID] Imperative summary`.
Apply the `agent-work` label, add `task:<TASK-ID>`, and fill in
`agent-work/PULL_REQUEST_TEMPLATE.md`.

Push the task branch and target the integration branch explicitly:

```bash
git push -u origin HEAD
gh pr create --base agent --title "[RES-01] Inventory existing components" \
  --body-file agent-work/PULL_REQUEST_TEMPLATE.md
```

Because the agent template is outside `.github/`, GitHub does not load it
automatically. Start the PR body from `agent-work/PULL_REQUEST_TEMPLATE.md`,
then complete every section with task-specific evidence. The PR base is
`agent` unless the task index requires a stacked base.

The PR body must identify:

- the task manifest and task-owned deliverables;
- each dependency and its merged PR;
- the architectural or numerical decisions made;
- exact validation commands and results;
- known limitations, follow-up tasks, and files intentionally out of scope.

Review comments and approval stay on GitHub. Do not mirror them into a task
manifest, report front matter, or checkpoint. Agents do not merge their own
pull requests. If review requests changes, update the same branch and PR rather
than opening another PR for the task.

## Pause, handoff, and concurrency

A paused task keeps its PR open and adds a checkpoint at
`agent-work/checkpoints/<TASK-ID>/<YYYY-MM-DD>-<slug>.md`. The checkpoint
records completed work, remaining work, command results, reference revisions,
and the next safe action.

Parallel tasks must use distinct branches, worktrees, and task-owned output
files. Serialize edits to shared interfaces, workspace manifests, and
consolidated design documents. If two tasks need the same file, either define a
stacked dependency in the task index or move the shared edit into its own task.
