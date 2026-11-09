# Decoder Tracker Agent Guide Implementation Plan

**Goal:** Make the decoder-tracker corpus lifecycle discoverable and safe for
maintainers and agents by pairing a crate-local guide with a project skill and
completing the manual-workflow handoff documentation.

## Scope

In scope:

- Add always-on rules and quick commands at the crate boundary.
- Add a discoverable skill for local investigation, shard merging, manual CI
  baseline refreshes, corpus snapshot updates, and README regeneration.
- Document how to apply the `decoder-baseline-update` workflow artifact.
- Link the new guidance from the crate README and architecture documentation.

Out of scope:

- Rust, CLI, workflow, shell, SQL, CSV schema, manifest, or baseline changes.
- RAW payload downloads, corpus refreshes, or full decoder runs.
- Changes to `TODO.md` or unrelated crates.

## Task Checklist

- [x] Record baseline agent behavior for a baseline-refresh scenario.
- [x] Add `crates/decoder-tracker/AGENTS.md`.
- [x] Add `.agents/skills/decoder-corpus-tracker/SKILL.md`.
- [x] Complete the workflow artifact and README handoff in
      `docs/decoder/corpus.md`.
- [x] Add discoverability links in the crate README and
      `docs/architecture.md`.
- [x] Forward-test the skill against the baseline scenario.
- [x] Run documentation validation and inspect the final diff.

## Files Expected to Change

- `crates/decoder-tracker/AGENTS.md` (new)
- `.agents/skills/decoder-corpus-tracker/SKILL.md` (new)
- `crates/decoder-tracker/README.md`
- `docs/decoder/corpus.md`
- `docs/architecture.md`
- `docs/plans/chore-decoder-tracker-agent-guide.md` (new)

## Validation

- Record a read-only baseline scenario before writing the skill, then repeat it
  with the skill and confirm the required setup, guard, artifact, baseline,
  and README steps are followed.
- Confirm every relative Markdown link resolves.
- Parse the skill frontmatter with `yq`; confirm its `name` matches the
  directory and its description starts with `Use when`.
- Run the decoder-tracker command help checks named by the guide.
- Regenerate the README with an unchanged baseline and confirm the generated
  marker region is byte-identical.
- Run `git diff --check`.
- Run `scripts/agent-validation.sh --docs-only`.
- Inspect the diff to confirm manifests, baselines, scripts, workflows, and
  generated README content are unchanged.

## Decisions and Notes

- The user selected both the crate guide and project skill, covering the full
  lifecycle through manual CI and corpus snapshots.
- `docs/decoder/corpus.md` remains the detailed schema and CI reference.
  `AGENTS.md` owns always-on rules, and the skill owns conditional procedures.
- Routine submodule initialization preserves the committed gitlink. The
  `--remote` form is reserved for a deliberate corpus snapshot refresh.
- RAW payloads remain transient and are never committed, cached, or uploaded.
- The pre-existing dirty `external/raw.pixls.us` checkout belongs to the user;
  this work runs in an isolated worktree and does not modify that state.
- This is an explicit maintenance request, so `TODO.md` remains unchanged.
- The read-only baseline evaluation found fragmented retrieval, routine use of
  the remote-refresh command, and an undocumented changed-artifact handoff.
- The forward evaluation with the skill passed after adding explicit final
  `git diff --check` and docs-only validation commands.
- The README badge/table regeneration was byte-identical with the committed
  baseline.
- `git diff --check`, decoder-tracker command help checks, skill frontmatter
  parsing, code-fence checks, relative-path checks, and
  `scripts/agent-validation.sh --docs-only` passed.
- Final review found three workflow ambiguities: the acceptance rerun could use
  an unpushed branch, snapshot commits omitted the submodule pointer, and
  manual decoder runs did not route remote-manifest drift to the snapshot
  update path. Documentation checks failed before the fixes and passed after
  all three paths were made explicit.
- Deferred minor wording: the artifact section assumes an existing baseline
  when listing three files, and `gh workflow run` does not reliably print the
  run ID directly.
