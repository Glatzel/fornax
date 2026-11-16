# Cheap Golden Decode Hash CI Implementation Plan

This archived plan documents the completed Cheap Golden Decode Hash CI work.

## Goal

Add a focused Ubuntu CI job that checks the committed decoder golden hashes
without building the whole workspace, fetching the full RAW corpus, or running
`compare-demosaic`.

## Scope

In scope:

- add a `golden` changed-path output and job to `.github/workflows/ci.yml`;
- fetch only the nine selected LFS payloads used by the golden manifest;
- cache the pinned submodule's fetched LFS objects by its gitlink SHA;
- build only `decoder-tracker` with the existing Rust setup action;
- run the golden gate on matching pull requests, pushes to `main`, and the
  weekly schedule;
- record measurement results in this plan and the pull request body.

Out of scope:

- changing golden files, hash values, decoder code, manifests, or the submodule
  pointer;
- running `compare-demosaic`;
- reducing the golden manifest below nine rows.

## Task Checklist

- [ ] Record the available baseline from the last ten `main` CI runs.
- [ ] Add the golden path filter and Ubuntu-only job.
- [ ] Initialize the pinned submodule as metadata only.
- [ ] Restore and save selected LFS objects under the pinned submodule SHA.
- [ ] Build and invoke only `decoder-tracker`.
- [ ] Validate workflow syntax and the unchanged nine-row local golden result.
- [ ] Open one pull request and measure matching and non-matching PR behavior.
- [ ] Measure the post-merge-equivalent `main` push behavior.

## Files Expected to Change

- `.github/workflows/ci.yml`
- `docs/_archive/plans/golden-corpus-ci.md`

## Validation

- Run `actionlint` for the workflow if available.
- Run the existing golden command locally:

  ```bash
  cargo run -q -p decoder-tracker -- hash \
    --manifest crates/decoder-tracker/golden/selected-manifest.csv \
    --check crates/decoder-tracker/golden/decode-hashes.csv
  ```

- Confirm the command reports `golden: unchanged (9 rows)`.
- Run `scripts/agent-validation.sh --docs-only`.
- Confirm `git diff --check`.
- Inspect the final diff to ensure no golden, decoder, manifest, or submodule
  pointer file changed.

## Decisions and Notes

- The checked-in `ci.yml` at `5ecf385` has no existing golden decode-hash job.
  The requested baseline therefore has no golden-job samples to report.
- The existing tracker command already fetches selected paths through Git LFS.
  The cache stores only the submodule's LFS object directory, keyed by the
  pinned gitlink SHA, so cache hits avoid re-downloading payloads while the
  checkout remains pointer-only until the gate materializes its nine files.
- raw.pixls.us replaces its snapshot history. The job therefore advances the
  pointer-only submodule to the advertised `master` snapshot before fetching
  the selected LFS object paths; the cache remains keyed by Fornax's pinned
  submodule SHA as requested.
- Classic branch protection reports no protection for `main`, so no no-op
  branch-protection companion is added.
