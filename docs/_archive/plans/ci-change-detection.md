# CI Path-Based Job Gating Implementation Plan

This archived plan documents the completed CI Path-Based Job Gating work.

## Goal

Add one change-detection job to `.github/workflows/ci.yml` and use its
outputs to run only the CI work affected by a push or pull request.

## Scope

In scope:

- Push and pull-request path detection.
- Conditions for `main`, benchmark, comparison, book-asset, and failure-gate
  jobs.
- Manual and scheduled event behavior.

Out of scope:

- Changes to `.github/workflows/ci-lefthook.yml`.
- Workflow trigger, concurrency, action pin, test, or benchmark changes.
- `TODO.md` and application code.

## Task Checklist

- [x] Create the branch and isolated worktree from `origin/main`.
- [x] Add the `changed` job and path group outputs.
- [x] Gate existing jobs and benchmark matrix rows.
- [x] Validate YAML, workflow expressions, and event scenarios.
- [x] Run documentation-only repository validation and inspect the diff.
- [x] Commit, push, and open pull request
  [#659](https://github.com/Glatzel/fornax/pull/659).

## Files Expected to Change

- `.github/workflows/ci.yml`
- `docs/_archive/plans/ci-change-detection.md`

## Validation

- Parse `.github/workflows/ci.yml` with `yq`.
- Run `actionlint` against `.github/workflows/ci.yml`.
- Check push, pull-request, manual, and scheduled conditions against the
  scenarios in this plan.
- Run `scripts/agent-validation.sh --docs-only`.
- Run `git diff --check`.

Validation results:

- `yq eval '.' .github/workflows/ci.yml`: passed.
- `actionlint -config-file /tmp/actionlint-fornax.yaml
  .github/workflows/ci.yml`: passed; `ubuntu-slim` was declared as the
  repository's known runner label because actionlint's bundled list omits it.
- `scripts/agent-validation.sh --docs-only`: passed.
- `git diff --check`: passed.

## Decisions and Notes

- GitHub Actions cannot create jobs at runtime. The implementation uses one
  detection job with named outputs and static downstream job gates.
- `main` runs on every push because it uploads Codecov coverage.
- Benchmark groups run on push only when their paths change.
- Other test jobs do not run on push.
- Pull requests run jobs only when their corresponding change output is true;
  Lefthook remains unchanged and PR-only through its existing caller.
- Manual dispatch preserves the current `main`, benchmark, and comparison
  behavior.
- Scheduled runs execute only `main`; the detector and other jobs remain
  skipped.
- The detector runs only for push and pull-request events. Downstream
  conditions account for the skipped detector on manual and scheduled runs.
