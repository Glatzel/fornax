# Publish Readiness Implementation Plan

This archived workspace-refactor record documents Publish Readiness Implementation Plan.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Produce a read-only report covering workspace membership, publishable-crate metadata, path dependencies, name availability, package contents, archive sizes, publish dry runs, and docs.rs feature behavior without changing code or publishing anything.

**Architecture:** Capture Cargo metadata and manifest facts as immutable evidence, then run read-only Cargo packaging and publish checks in dependency order. Record every result, including expected failures caused by unpublished path dependencies, in one workspace-refactor report.

**Tech Stack:** Cargo, crates.io HTTP endpoints, docs.rs metadata semantics, Git worktrees, repository documentation validation.

**Spec:** `docs/_archive/workspace-refactor/publish-readiness-plan.md`

## Global Constraints

- Start from current `origin/main` after PR #697 and use branch `chore/publish-readiness` in `.worktrees/chore-publish-readiness`.
- Make documentation-only changes; do not edit manifests, Cargo features, dependencies, lockfiles, versions, crate names, golden files, production code, or the Python API.
- Do not publish, reserve names, recommend publication timing, or decide crate names or versions.
- Inventory all workspace members, including `publish = false`; explicitly note that `fornax-py` is excluded from the workspace.
- Treat name checks experiencing network or rate-limit failures as `[UNVERIFIED]`.
- Keep unpublished path-dependency failures as evidence and do not work around them.
- Do not compile `compare-demosaic`, run lefthook, or run benchmarks.
- Do not block on local LFS materialization; CI is authoritative for LFS-backed checks.

## Review Focus

- A `publish = false` member is omitted from the inventory or mistaken for publishable.
- A path dependency is reported as versioned when it has only a local path source.
- An archive size, file count, or leak finding is inferred rather than measured.
- A docs.rs feature set is assumed when package metadata is absent.
- A dry-run failure is hidden or converted into a proposed fix instead of being recorded as evidence.

## Task 1: Inventory, metadata, dependencies, and names

**Files:**
- Create: `docs/_archive/workspace-refactor/publish-readiness-report.md`

**Interfaces:**
- Consumes: `cargo metadata --no-deps`, package manifests, crates.io crate endpoints.
- Produces: inventory, publishability classification, required metadata audit, path-dependency audit, version compatibility note, and dated name-availability table.

- [x] **Step 1: Capture the package inventory and publishability**

Run:

```bash
cargo metadata --no-deps --format-version 1 > /tmp/fornax-publish-metadata.json
jq -r '.packages[] | [.name, .version, (.publish | tostring), .manifest_path] | @tsv' /tmp/fornax-publish-metadata.json
```

Expected: 12 workspace members at `0.0.17`; `compare-demosaic` and `decoder-tracker` have `publish = false`; the other 10 are candidates for Cargo publishing. Record that `crates/fornax-py` is excluded by the root manifest.

- [x] **Step 2: Audit publishable package metadata**

Inspect each publishable manifest and metadata for description, license, repository, readme, `rust-version`, version, `[features]`, and `package.metadata.docs.rs`. Expected: absent required publication fields are blockers and are reported without editing manifests.

- [x] **Step 3: Audit normal and build path dependencies**

Run:

```bash
jq -r '.packages[] | select((.publish | length) == 0 or .publish == null) | .name as $package | .dependencies[] | select((.kind == null or .kind == "build") and .path != null) | [$package, .name, .req, .path, (.kind // "normal")] | @tsv' /tmp/fornax-publish-metadata.json
```

Expected: all workspace path dependencies currently have wildcard `*` requirements and no registry version pin. Separate path-only dependencies from path plus version entries and report blockers without fixing them.

- [x] **Step 4: Record the version compatibility caveat**

State that the workspace is currently `0.0.17` and Cargo semver permits breaking changes between `0.0.x` patch releases.

- [x] **Step 5: Check crate-name availability**

Query `https://crates.io/api/v1/crates/<crate-name>` for all 10 publishable names, record the UTC query date, interpret `200` as existing and `404` as available, and mark rate limits or network failures `[UNVERIFIED]`.

- [x] **Step 6: Start the report**

Create the report with sections for scope/method, inventory, metadata completeness, path dependencies, name availability, package evidence placeholders, dry runs, docs.rs features, blockers, and `[UNVERIFIED]` items. Commit the inventory.

## Task 2: Package evidence, dry runs, and completion

**Files:**
- Modify: `docs/_archive/workspace-refactor/publish-readiness-report.md`
- Modify: `docs/_archive/workspace-refactor/publish-readiness-implementation.md`

**Interfaces:**
- Consumes: publishable crate list from Task 1.
- Produces: file counts, uncompressed sizes, leak audit, archive bytes, dependency-ordered dry-run results, docs.rs feature sets, and completed validation evidence.

- [x] **Step 1: Capture package file lists and uncompressed sizes**

For each publishable crate, run `cargo package --list -p <crate>`, save the output under `/tmp`, count entries, sum existing file sizes, and scan for `external/`, `temp/`, RAW extensions, generated samples, or local-only development assets.

- [x] **Step 2: Capture archives**

Run `cargo package --no-verify -p <crate>` and record each `target/package/<crate>-0.0.17.crate` byte size or the exact failure. Do not work around failures.

- [x] **Step 3: Run dependency-ordered publish dry runs**

Use: `fornax-raw-core`, `fornax-codec`, `fornax-tiff`, `fornax-dng`, `fornax-arw`, `fornax-raw-pipeline`, `fornax-devtool`, `fornax-decoder`, `fornax-raw`, `fornax-cli`.

Run `cargo publish --dry-run -p <crate>` and record pass/fail plus the first actionable error for each crate.

- [x] **Step 4: Derive docs.rs feature behavior**

For each crate, record `[features]`, `package.metadata.docs.rs` values, `all-features`, and `no-default-features`. If docs.rs metadata is absent, cite docs.rs default behavior rather than assuming a feature set.

- [x] **Step 5: Complete the report and branch ledger**

Fill every required report section and both dependency trees. Mark blockers and unresolved evidence accurately. Update this plan's checklist and decision notes with actual outcomes.

- [x] **Step 6: Validate**

Run:

```bash
scripts/agent-validation.sh --docs-only
git diff --check
cargo tree -p fornax-codec -e normal --depth 1
cargo tree -p fornax-tiff -e normal --depth 1
cargo run -q -p decoder-tracker -- hash \
  --manifest crates/decoder-tracker/golden/selected-manifest.csv \
  --check crates/decoder-tracker/golden/decode-hashes.csv
```

Expected: docs-only validation passes, diff check is clean, dependency trees retain their current direct dependencies, and the golden check reports `golden: unchanged (9 rows)`.

- [x] **Step 7: Commit and open one PR**

Commit documentation only, push the branch, and open `docs: report publish readiness` against `main`. Stop after creation without watching, polling, or merging.

## Files Expected to Change

- `docs/_archive/workspace-refactor/publish-readiness-implementation.md`
- `docs/_archive/workspace-refactor/publish-readiness-report.md`

## Validation

- `scripts/agent-validation.sh --docs-only`
- `git diff --check`
- `cargo tree -p fornax-codec -e normal --depth 1`
- `cargo tree -p fornax-tiff -e normal --depth 1`
- Golden decoder hash command from Step 6
- Package-list, archive, metadata, name-availability, and publish-dry-run commands captured in the report

## Decisions and Notes

- This report is evidence and blocker inventory, not a publication plan.
- The current workspace snapshot is `0.0.17`; no version or manifest changes are authorized.
- Expected failures from unpublished path dependencies are reported as evidence.
- Network-dependent results are `[UNVERIFIED]` when they cannot be observed.
- CI is authoritative for LFS-backed checks; local LFS materialization is not a gate.
- Inventory evidence: 12 workspace members, 10 Cargo-publishable crates, and
  `fornax-py` excluded from the workspace.
- Name checks: all 10 candidate names returned HTTP 404 on 2026-11-10.
- Package and dry-run checks: all 10 stopped before archive creation because
  path or Git dependencies lacked registry version requirements.
- Golden hash command stopped at local LFS materialization; CI remains the
  authority for that check.
