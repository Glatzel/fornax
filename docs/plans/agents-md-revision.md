# Plan: Revise the root AGENTS.md, add architecture docs and project skills

Work type: maintenance (not a `TODO.md` item). Branch
`chore/agents-md-revision`, worktree `.worktrees/chore-agents-md-revision`,
one semantic pull request into `main`. Do not change `TODO.md`.

## Goal

Rewrite the root `AGENTS.md` as a short, tool-agnostic map that any coding
agent can follow. Move durable architecture detail to `docs/architecture.md`,
comparison-specific rules to `crates/compare-demosaic/AGENTS.md`, and the
repeatable comparison procedure to
`.agents/skills/add-demosaic-comparison/SKILL.md`.

The new root must let an agent select a `TODO.md` item, create a worktree,
write a plan when needed, implement, validate, and open one pull request. It
must also point to architecture, crate-specific rules, plans, and skills.

## Scope

In scope:

- Rewrite the root `AGENTS.md`.
- Add `docs/architecture.md` and this committed plan.
- Merge current comparison-specific rules into
  `crates/compare-demosaic/AGENTS.md`.
- Add the `add-demosaic-comparison` project skill.
- Make only the `.gitignore` change needed to ensure new project files are not
  ignored.

Out of scope:

- Source code, CI workflows, or `TODO.md` changes.
- Tool-specific instruction files or product-specific commands/settings.
- Speculative design notes or additional skills.

## Files expected to change

- `AGENTS.md`
- `crates/compare-demosaic/AGENTS.md`
- `docs/architecture.md`
- `docs/plans/agents-md-revision.md`
- `.agents/skills/add-demosaic-comparison/SKILL.md`
- `.gitignore` only if an existing rule hides a required new file

## Constraints

- Keep the repository root clean. The only agent-related root files are
  `AGENTS.md` and `TODO.md`.
- Project skills live in `.agents/skills/<name>/SKILL.md`. Never edit or delete
  skills installed and managed by pixi; only add project skill directories.
- `TODO.md` is human-owned. Agents never add items; they propose new items in
  the report or pull request description.
- Plans live in `docs/plans/`, are committed with the work, and never live in
  `agent-work/`.
- Keep instructions tool-agnostic.

## Task checklist

- [x] 1. Create the branch and worktree from `origin/main` (2026-11-01).
- [x] 2. Inventory the current files and record findings in Notes
  (2026-11-01).
- [x] 3. Build the disposition table (every old rule: keep, move, or drop)
  (2026-11-01).
- [x] 4. Write `docs/architecture.md` (2026-11-01).
- [x] 5. Move comparison-specific rules into
  `crates/compare-demosaic/AGENTS.md` and create the project skill(s).
- [x] 6. Write the new root `AGENTS.md` (2026-11-01; 118 lines).
- [x] 7. Check `.gitignore` for `agent-work/` and for accidental ignoring of
  new files (2026-11-01).
- [x] 8. Run validation and fix problems (2026-11-01).
- [ ] 9. Commit (this plan included), push, open the pull request, report the
  URL, and stop.

## Steps

### 1. Branch and worktree

Create the requested branch and worktree from current `origin/main`, and do all
work inside it:

```bash
git fetch origin
git worktree add -b "chore/agents-md-revision" \
  ".worktrees/chore-agents-md-revision" origin/main
```

### 2. Inventory

Read and verify the root and comparison instructions, backlog format,
`.gitignore`, semantic title types, CI workflows, existing docs, `.agents/`,
pixi configuration, workspace layout, referenced paths and commands, and the
current remote heads. Check every retained claim against the repository.

### 3. Disposition table

Classify every distinct old root rule as keep in root, move to architecture,
move to the comparison guide, move to the skill, or drop with a reason. Do not
silently lose a rule. Put the complete table in the pull request description.

Obsolete warnings about per-task manifests, `STATUS.md`,
`agent-work/AGENTS.md`, design gates, and checkpoint trees may be dropped
because the rewritten instructions no longer describe or recreate that
workflow.

### 4. Architecture

Write a roughly 100-line `docs/architecture.md` sourced from the workspace,
crate sources, and README. Include:

1. Purpose and users.
2. Crate roles and internal dependencies.
3. Decode, preprocess, and demosaic boundaries, inputs, outputs, and code
   locations.
4. The conventional 2x2 RGB Bayer support boundary.
5. Rust, Python, and other interfaces that actually exist.
6. A comparison-tooling summary linked to its agent guide.

Add another design note only when it records existing design that is currently
undocumented or buried in the old root instructions.

### 5. Comparison rules and project skill

Move "Accepted outliers", CLI details, the comparison CI workflow, and
reference-source setup into `crates/compare-demosaic/AGENTS.md`. Read that file
first, merge without duplication, and prefer current code where text disagrees.

Create `.agents/skills/add-demosaic-comparison/SKILL.md` for the repeatable
procedure. Its frontmatter `name` must match the directory. Describe when to
use it, use imperative verified steps and exact commands, stay near 80 lines,
and link to the comparison guide instead of duplicating waiver or CLI rules.
Leave a pointer to the skill in the comparison guide. Do not create another
skill unless another repeatable procedure genuinely requires one.

### 6. Root AGENTS.md

Rewrite the root in this order, at most 120 lines and ideally near 100:

1. Overview and instruction precedence.
2. Project map and permanent stage/support boundaries.
3. Choosing work: task, issue, and chore branch/worktree/PR rules.
4. Shared workflow.
5. Plans.
6. Validation.
7. Pull request handoff.
8. Boundaries.
9. Where to look next.

For plans, require:

- Write one only for non-trivial work: several files or crates, a design
  decision, or more than a short session.
- Use `docs/plans/<lowercase-id>-<slug>.md` for tasks,
  `fix-<number>-<slug>` for issues, and `chore-<slug>` for chores.
- Commit it with the work; keep it updated while active and treat it as
  historical after merge.
- Include Goal, Scope (in and out), Task checklist, Files expected to change,
  Validation, and Decisions and notes.
- Derive the detailed checklist from the code and one-line backlog item without
  expanding `TODO.md`.
- Make it self-contained for a reader with only the working tree and the plan.

### 7. Gitignore

Confirm `agent-work/` is ignored. Confirm new `docs/` and
`.agents/skills/add-demosaic-comparison/` files are not ignored with
`git status` and `git check-ignore -v`. Make no unrelated `.gitignore` changes.

### 8. Validation

Run:

```bash
git diff --check
```

Also verify:

- Root `AGENTS.md` is at most 120 lines.
- Changed files contain no agent-product names and no named tool-specific
  instruction-file reference.
- Relative links and referenced paths exist; cheap commands such as
  `cargo run --release -p compare-demosaic -- --help` run as written.
- Each skill's frontmatter name matches its directory and has a description.
- Architecture statements match the code.
- Configured Markdown lint passes.
- `git diff --stat origin/main` shows no changes to `TODO.md`, source code, CI,
  or pre-existing `.agents/` files.

The full Rust test suite is not required because this change does not modify
Rust source. Report that fact rather than claiming unrun checks passed.

### 9. Pull request

Commit with a semantic title, push, and open one semantic pull request into
`main`. Include a summary, the complete disposition table, exact validation
results, and assumptions. Report the URL and stop without watching, polling,
merging, or closing it.

## Acceptance

- Root instructions are fully rewritten within the line limit and contain no
  tool-specific wording or retired workflow artifacts.
- Every still-relevant old rule is preserved in one new location or explicitly
  dropped in the disposition table.
- Work types, plan convention, validation commands, and handoff rules appear
  once at the root.
- New files are limited to this plan, `docs/architecture.md`, the comparison
  skill, and any justified design note.
- All claimed validation actually ran; skipped checks are named.
- No changes to source code, CI workflows, `TODO.md`, or pre-existing managed
  `.agents/` files.

## Decisions

- Keep `TODO.md` as the coarse human-owned backlog and put generated detail in
  committed per-item plans.
- Commit plans with their pull request without an active/completed split.
- Keep permanent pipeline and support invariants in the root; put architecture
  explanation in `docs/architecture.md`.
- Use `.agents/skills/<name>/SKILL.md` for project skills and leave managed
  installations alone.
- Add no tool-specific entry files.

## Notes

- 2026-11-01: `origin/main` was `4a3325a`. The requested branch and worktree
  were created from it.
- 2026-11-01: Inventory found no tracked `.agents/` or `docs/` files, and no
  pixi configuration currently installing skills. The new project skill can
  therefore be added without touching managed files.
- 2026-11-01: `.gitignore` already contains `/agent-work/` and does not ignore
  `.agents/` or `docs/`.
- 2026-11-01: `refactor/raw-pipeline-v2` no longer exists on the remote, so the
  old guard against copying its files will be dropped.
- 2026-11-01: Current `compare-demosaic` CLI has only `run`, `diff`, and
  `readme`; there is no `--update`. CI uploads comparison artifacts only on
  failure. The existing crate guide matches these current facts.
- 2026-11-01: The old comparison procedure names a `src/cases/` directory and
  generated `RESULTS.md`; both have been replaced by `src/cases.rs` and
  `README.md` regions. The new skill uses the current paths and commands.
- 2026-11-01: The root README still lists several historical crate names. This
  change does not rely on that component list; the architecture document uses
  the current `Cargo.toml` manifests and crate sources.
- 2026-11-01: The supplied intended plan filename is preserved as
  `docs/plans/agents-md-revision.md`. The new root instruction states
  `chore-<slug>` for future chore plans; this file predates that convention.
- 2026-11-01: Validation passed for `git diff --check`, the repository
  Markdown lint (5 files, 0 issues), the project skill validator, all relative
  links, documented path existence, the 120-line root limit, and the
  tool-specific-name scan. The comparison CLI help command passed after the
  documented setup. It emitted an existing darktable RCD adapter
  `-Wmaybe-uninitialized` build warning, but exited successfully.
- 2026-11-01: The full Rust test suite was not run because this change does not
  modify Rust source; the comparison binary was built for the CLI check.
