# Publish Readiness Report Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Produce a read-only publish-readiness report for every workspace crate that Cargo would allow publishing.

**Architecture:** Query Cargo metadata, crates.io, package contents, and publish dry runs without changing manifests, versions, dependency declarations, or remote state. Record all evidence and failures in one report.

**Tech Stack:** Cargo 1.99.1, crates.io HTTP API, shell file inspection, Phase B-C handoff rules.

**Spec:** `docs/plans/workspace-refactor/phase-b-c-handoff.md`

## Global Constraints

- Use branch `chore/publish-readiness` and worktree
  `.worktrees/chore-publish-readiness`.
- Use one worktree and one pull request.
- Keep the branch plan at `docs/plans/chore-publish-readiness.md`.
- Do not publish anything.
- Do not change crate versions, metadata, manifests, lockfile, or source code.
- Do not decide whether or when crates should be published.
- Do not compile `compare-demosaic`.
- Do not edit golden files.
- If network access is unavailable, mark affected rows `[UNVERIFIED]`.

## Review Focus

- Path-only dependencies must be visible even when local builds succeed.
- Package contents must not include `external/`, RAW fixtures, or sample files.
- A dry-run failure caused by unpublished path dependencies must be reported, not worked around.
- Every `0.0.x` compatibility caveat must be tied to the current `0.0.17` workspace version.
- docs.rs feature behavior must be derived from manifest metadata or docs.rs documentation, not assumed.

---

### Task 1: Inventory and metadata audit

**Files:**
- Create: `docs/plans/workspace-refactor/publish-readiness.md`

**Interfaces:**
- Consumes: current workspace Cargo metadata.
- Produces: crate inventory, publishability classification, and metadata table.

- [ ] **Step 1: Capture the workspace inventory**

Run:

```bash
cargo metadata --no-deps --format-version 1 > /tmp/fornax-publish-metadata.json
jq -r '.packages[] | [.name, .version, .publish, .manifest_path] | @tsv' \
  /tmp/fornax-publish-metadata.json
```

List every workspace member. Treat `publish = false` as non-publishable.
Explicitly note that `fornax-py` is excluded from the workspace.

- [ ] **Step 2: Audit required metadata**

For every publishable crate, record:

- description;
- license;
- repository;
- readme;
- `rust-version`;
- current version;
- docs.rs configuration.

Record missing values as failures in the report. Do not fill them in.

- [ ] **Step 3: Audit path dependencies**

For every publishable package, list normal/build dependency entries containing
a `path` source and whether they also contain a registry version requirement.
Production path dependencies without versions are publish blockers unless the
dependency is not part of the package being published.

- [ ] **Step 4: Check crate-name availability**

For each publishable crate name:

```bash
curl -sS -o /dev/null -w '%{http_code}\n' \
  "https://crates.io/api/v1/crates/<crate-name>"
```

Interpret `200` as existing and `404` as available at query time. Record the
query date and any rate-limit or network failure as `[UNVERIFIED]`.

- [ ] **Step 5: Record the compatibility caveat**

State that the workspace currently uses `0.0.17` and that under Cargo semver,
every `0.0.x` patch version may contain breaking changes.

- [ ] **Step 6: Commit the inventory**

```bash
git add docs/plans/workspace-refactor/publish-readiness.md
git commit -m "docs: record publish readiness inventory"
```

### Task 2: Inspect packages and run dry releases

**Files:**
- Modify: `docs/plans/workspace-refactor/publish-readiness.md`

**Interfaces:**
- Consumes: Task 1 crate inventory.
- Produces: file-list, package-size, leak, and dry-run results.

- [ ] **Step 1: Capture each package file list**

For each publishable crate:

```bash
cargo package --list -p <crate> > /tmp/fornax-<crate>-package-files.txt
```

Record the file count and total uncompressed size. Search the file list for:

```text
^external/
^temp/
\.(dng|arw|cr2|nef|raf|rw2|orf)$
```

Report any external tree, RAW payload, generated sample, or local-only
development asset.

- [ ] **Step 2: Capture package archive sizes**

For each crate:

```bash
cargo package --no-verify -p <crate>
```

Record `target/package/<crate>-<version>.crate` size with `stat`. If packaging
fails, record the exact failure and continue with the next crate.

- [ ] **Step 3: Run publish dry runs in dependency order**

Derive dependency order from metadata. For the current workspace, verify and
use this order unless the metadata shows a new dependency:

```text
fornax-raw-core
fornax-codec
fornax-tiff
fornax-dng
fornax-arw
fornax-raw-pipeline
fornax-devtool
fornax-decoder
fornax-raw
fornax-cli
```

Run:

```bash
cargo publish --dry-run -p <crate>
```

Record pass/fail and the first actionable error per crate. A failure caused by
dependencies absent from crates.io is expected evidence, not a reason to
publish or work around the check.

- [ ] **Step 4: Record docs.rs features**

For each crate, inspect `package.metadata.docs.rs` and `[features]`. Record:

- configured docs.rs features;
- whether `all-features` is enabled;
- whether default features are disabled;
- the resulting build set according to docs.rs configuration semantics.

If no docs.rs metadata exists, state that and cite the docs.rs default behavior.

- [ ] **Step 5: Complete the report**

Use sections for:

- crate inventory and publishability;
- metadata completeness;
- path dependency versions;
- crate-name availability;
- package contents and sizes;
- publish dry-run results;
- docs.rs feature sets;
- blockers and `[UNVERIFIED]` items.

Do not recommend a publication date or version change.

- [ ] **Step 6: Run documentation and global checks**

```bash
scripts/agent-validation.sh --docs-only
git diff --check
cargo tree -p fornax-codec -e normal --depth 1
cargo tree -p fornax-tiff -e normal --depth 1
cargo run -q -p decoder-tracker -- hash \
  --manifest crates/decoder-tracker/golden/selected-manifest.csv \
  --check crates/decoder-tracker/golden/decode-hashes.csv
```

Expected: documentation checks pass, dependency trees are unchanged, and
`golden: unchanged (9 rows)`.

- [ ] **Step 7: Commit the completed report**

```bash
git add docs/plans/workspace-refactor/publish-readiness.md
git commit -m "docs: report publish readiness"
```
