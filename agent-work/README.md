# Agent Workspace

This directory contains the durable instructions, task contracts, evidence, and
checkpoints for agent-maintained work. Root [`AGENTS.md`](../AGENTS.md) is the
discovery entry point; this file explains the layout.

```text
agent-work/
├── AGENTS.md                         # operating instructions
├── README.md                         # this index
├── workflows/
│   └── raw-pipeline.md               # authoritative pipeline workflow
├── tasks/
│   ├── README.md                     # manifest schema and execution protocol
│   └── raw-pipeline/
│       ├── README.md                 # task index and dependency graph
│       └── <TASK-ID>.toml            # exactly one task contract per file
├── research/
│   ├── README.md
│   ├── <TASK-ID>-<slug>.md           # task-owned evidence reports
│   └── notes/<TASK-ID>.md            # task-owned synthesis notes
├── design/
│   ├── raw-pipeline.md               # DES-007 consolidated design
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
- The task's GitHub pull request is the only review record. Open means in
  progress or review; merged means accepted; closed without merge means
  abandoned or superseded.

Do not add a `review`, `approved`, or `done` status to task TOML files, reports,
or checkpoints. Those states duplicate and can conflict with GitHub.

## Task lifecycle

1. Select a task whose dependencies are represented by merged pull requests.
2. Create `.worktrees/<id>-<slug>` on `task/<id>-<slug>` from the current
   integration base.
3. Work only inside the manifest's scope and deliverables.
4. Commit, push, and open one pull request for the task.
5. Iterate in that pull request. The user reviews and approves it there.
6. An agent never merges or closes the review pull request.

The repository does not maintain a second dashboard of PR states. Use GitHub's
pull-request search, labels, and branch names. The task index records the
planned dependency graph, not live status.

See [`tasks/README.md`](tasks/README.md) for the manifest schema and exact Git
commands.
