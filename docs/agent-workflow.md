# Agent workflow details

Read this when starting a branch, writing a plan, changing validation, or
preparing a pull request. The root `AGENTS.md` remains authoritative.

## Choosing work

| Request | Branch | Worktree | Pull request |
| --- | --- | --- | --- |
| `TODO.md` item selected by bracketed ID | `task/<lowercase-id>-<slug>` | `.worktrees/<lowercase-id>-<slug>` | Include the ID and remove that item from `TODO.md`; keep it if work is incomplete or blocked. |
| GitHub issue number | `fix/<number>-<slug>` | `.worktrees/fix-<number>-<slug>` | Read only that issue with `gh issue view <number>`, add regression coverage, and include `Fixes #<number>`. |
| Explicit request not in `TODO.md` | `chore/<slug>` | `.worktrees/chore-<slug>` | Leave `TODO.md` untouched unless the request names an item. |

Create the worktree from current `origin/main`:

```bash
git fetch origin
git worktree add -b "<branch>" ".worktrees/<worktree>" origin/main
```

## Plans

Plan nontrivial work: several files or crates, a design decision, or more than
a short session. Store it as:

- Task: `docs/plans/<lowercase-id>-<slug>.md`
- Issue: `docs/plans/fix-<number>-<slug>.md`
- Chore: `docs/plans/chore-<slug>.md`

Every plan needs Goal, Scope (in/out), Task checklist, Files expected to
change, Validation, and Decisions and notes. Make it self-contained without
expanding `TODO.md`. Commit it with the PR and keep it current while work is
open; it stops being authoritative after merge.

## Benchmark matrix and validation details

Decoder benchmark matrix entries compile with `--no-default-features`. All
native vendor formats remain available because there are no per-format
features; this only disables independent infrastructure such as mmap.

`scripts/agent-validation.sh` runs:

- `git diff --check` and `scripts/check-doc-links.py`;
- raw-image/corpus setup when missing;
- corpus manifest and structural golden-set checks;
- decoder-tracker DuckDB tests;
- workspace tests, codec/vendor tests, nightly formatting, docs, and clippy;
- decoder benchmark compilation;

Use `--fix` for nightly rustfmt and machine-applicable clippy fixes. Use
`--docs-only` for documentation/context-only changes. Markdown lint is CI-only.
The CI golden job materializes the pinned RAW payloads and validates every
committed test hash; routine local validation never fetches those payloads.

Hard-link the main checkout's large raw-image fixtures in linked worktrees:

```bash
mkdir -p external
rsync -a --link-dest="<main-checkout>/external/raw-images" \
  "<main-checkout>/external/raw-images/" external/raw-images/
```

Initialize the raw.pixls.us corpus as a pointer-only submodule in each
worktree. Do not hard-link its working tree: submodule Git metadata and
transient LFS materialization are per-worktree. Never edit hard-linked input and
never download the RAW corpus again merely because a worktree is new.

```bash
GIT_LFS_SKIP_SMUDGE=1 git -c filter.lfs.required=false \
  submodule update --init external/raw.pixls.us
```

## Quiet tooling

Use the generic wrapper for commands whose successful output is much larger
than the decision it supports:

```bash
scripts/agent-run.sh --label bench -- cargo bench -p fornax-codec
```

It captures stdout and stderr to `target/agent-logs/`, prints the log path,
exit status, duration, and the last 80 lines. `--tail N` changes the tail and
`FORNAX_AGENT_LOG_DIR` changes the directory. The wrapper preserves the child
exit code and full log, so use the log when the tail lacks the failure.

Keep raw commands where the output is an interface: CI artifact generation,
JSON/text consumed by another command, or short tracker operations whose
defaults are already bounded. Criterion has no stable machine summary here, so
the wrapper is the safe common mechanism; command-specific output modes would
add maintenance for little benefit.

## Pull request handoff

```bash
git add <paths>
git commit -m "<type>: <summary>"
git push -u origin HEAD
gh pr create --base main --title "<type>: <summary>" --body "<body>"
```

Use a title accepted by `.github/semantic.yml`. Put the task ID, issue
reference, validation commands, failures, and skipped checks in the body.
Report the PR URL. Only the user merges, watches, closes, or abandons it.

## Reference knowledge

- Preserve reusable algorithm and implementation research in
  `docs/reference/<category>/<topic>.md`, committed with the code it informs.
- Record authoritative links with pinned versions or commits, the behavior
  they support, implementation differences, and licence constraints.
- Link source modules or generated docs to notes whose invariants are easy to
  regress.
- Restricted-licence sources are approach references only. Write original
  Rust; never copy LGPL/GPL-family or other incompatible code.

## Where to look

- Architecture and pipeline boundaries: `docs/architecture/overview.md`
- Decoder ownership and validation: `docs/architecture/decoder.md`
- Decoder benchmarks: `docs/reference/decoders/operations/benchmark.md`
- Corpus rules: `docs/reference/decoders/operations/corpus.md`
- Manual tracker: `crates/decoder-tracker/AGENTS.md`
- Demosaic research: `docs/reference/demosaic/README.md`
- Book: `book/README.md` and `book/AGENTS.md`
- Plans: `docs/plans/`
- Algorithm research: `docs/reference/`
- Procedures: `.agents/skills/`

For the `Decoder corpus` workflow, leave `run_duckdb_tests` at its default
unless explicitly requested, and set `filter` to the requested path glob. Full
tracker runs are manual-only; never commit or upload RAW payloads.
