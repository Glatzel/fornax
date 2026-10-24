# Agent Workspace

This directory contains the durable instructions, task contracts, evidence, and
checkpoints for agent-maintained work. Root [`AGENTS.md`](../AGENTS.md) is the
discovery entry point; this file explains the layout.

```text
agent-work/
├── AGENTS.md                         # operating instructions
├── README.md                         # this index
├── PULL_REQUEST_TEMPLATE.md          # task PR body template
├── workflows/
│   └── raw-pipeline.md               # authoritative pipeline workflow
├── tasks/
│   ├── README.md                     # manifest schema and execution protocol
│   ├── STATUS.md                     # task-status snapshot
│   └── raw-pipeline/
│       ├── README.md                 # task index and dependency graph
│       └── <TASK-ID>.toml            # exactly one task contract per file
├── research/
│   ├── README.md
│   ├── <TASK-ID>-<slug>.md           # task-owned evidence reports
│   └── notes/<TASK-ID>.md            # task-owned synthesis notes
├── design/
│   └── raw-pipeline/
│       ├── README.md
│       └── <TASK-ID>-<slug>.md       # task-owned design proposals
├── checkpoints/
│   ├── README.md
│   ├── workflow/
│   └── <TASK-ID>/<date>-<slug>.md
└── references/
    ├── README.md
    └── <pinned upstream sources>
```

## Sources of truth

- The workflow defines the pipeline boundary and required gates.
- Each task TOML defines scope, dependencies, deliverables, acceptance
  criteria, and validation.
- The task-owned report contains evidence and conclusions.
- `tasks/STATUS.md` is the human-readable task-status snapshot. A task is
  marked `Done` when its deliverable is complete and its task PR is open.
- `../docs/raw-pipeline/` is the normative persistent design. Design proposals
  under `agent-work/design/` retain rationale and review history.
- The task's GitHub pull request is the only review record. Open means in
  progress or review; merged means accepted; closed without merge means
  abandoned or superseded.

GitHub only loads pull-request templates from `.github/`.
`PULL_REQUEST_TEMPLATE.md` is specific to agent tasks, so copy it into the PR
body manually when opening a task PR.

Do not add a `review`, `approved`, or `done` status to task TOML files, reports,
or checkpoints. Those states duplicate and can conflict with GitHub.

## RAW decoder direction

The planned implementation reuses existing RAW decoders behind a `fornax`-owned
adapter contract. The core owns the universal raw representation, backend
capabilities, deterministic backend selection, structured errors, and
provenance; backend crates own calls into their existing decoder libraries or
conversion tools.

The initial candidates are:

- LibRaw as the broad existing C/C++ decoder backend;
- dnglab/rawler as the pure Rust decoder backend;
- `dnc` as an Adobe-DNG conversion bridge that composes with LibRaw or
  dnglab/rawler; and
- darktable/rawspeed only through an isolated decoder boundary that returns
  sensor-domain data before darktable's `rawprepare` pipeline.

Do not introduce a first-party RAW/TIFF decoder as the initial implementation.
The detailed evidence and candidate boundaries are owned by the RES-09 report;
public APIs remain gated by synthesis and design.

## Task lifecycle

1. Select a task whose dependencies are represented by merged pull requests.
2. Create `.worktrees/<id>-<slug>` on `task/<id>-<slug>` from the current
   integration base.
3. Work only inside the manifest's scope and deliverables.
4. Commit, push, and open one pull request with a semantic title such as
   `docs(res-01): inventory existing components`. Mark the task `Done` in
   `tasks/STATUS.md` and add the PR link.
5. Report the PR URL and stop. Do not watch, poll, or wait for checks or
   review.
6. The user initiates the next step: request local changes or merge the PR.
   Apply requested changes on the same branch and PR unless the user asks for a
   local-only result.
7. An agent never merges or closes the task pull request.

The status snapshot is for task tracking. GitHub pull-request search, labels,
branches, and merged state remain authoritative for review and dependency
gates.

See [`tasks/README.md`](tasks/README.md) for the manifest schema and exact Git
commands.
