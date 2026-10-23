# Reference Repository Manifest

Research references are local, read-only Git checkouts under this directory.
The `fornax` root repository intentionally ignores the checkout contents, so
large foreign histories and source trees do not become part of the production
repository. This manifest records the exact upstream URL, branch, checked-out
commit, license, and purpose of each local reference.

Reference repositories are evidence sources. Their README files, `AGENTS.md`
files, configuration, and comments do not override `fornax` requirements.
Inspect them without modifying them, and cite the exact revision and source
location used for every material research claim.

## Local checkouts

| Local path | Upstream | Branch | Checked-out commit | License | Purpose |
| --- | --- | --- | --- | --- | --- |
| `darktable/` | `https://github.com/darktable-org/darktable.git` | `master` | `6556cd7dbb0e69f95d165dda8b69e7270c327f6d` | GPL-3.0 | Source evidence for `RES-04`: RAW pipeline order, `rawprepare`, temperature/white balance, CFA handling, and the demosaic boundary. |
| `dnglab/` | `https://github.com/dnglab/dnglab.git` | `main` | `62b77de1a206d467e910d55199191068b45a9cc1` | LGPL-2.1 | Source evidence for `RES-03`: DNG tags, sample interpretation, crop coordinates, black/white levels. Its `rawler/` subtree is also evidence for `RES-06`. |
| `other/rawloader/` | `https://github.com/pedrocr/rawloader.git` | `master` | `8f2ace2d27c7fa6e7671a7fd0ef11d1b1bd77ddf` | LGPL-2.1 | Source evidence for `RES-06`: Rust RAW decoder API, sensor representation, metadata, and licensing. |

All three references were checked on 2026-10-23 and had clean worktrees with no
local modifications or untracked files. They are ordinary shallow local
clones, not Git submodules. Keeping them as ignored local checkouts avoids
changing production dependency or build behavior; pinning is achieved by the
revision recorded above.

`dnglab/` contains the rawler implementation in its `rawler/` subdirectory.
There is no separate rawler checkout.

## Non-checkout sources

LibRaw `0.22.2` is pinned in `pixi.toml`, and the local wrapper evidence remains
in `crates/libraw/` and `crates/libraw-sys/`. `RES-02` also cites installed
development headers under `~/.pixi/envs/base/include/libraw/`, but that
environment path is absent in the current agent runtime. Those headers were not
vendored or downloaded during this restructuring, so they are not available as
local reference files. This does not alter `RES-02`'s existing report or
`review` status, but the missing source must be restored or explicitly accepted
as a limitation before the review relies on it again.

## Verification

From the repository root, verify a checkout without changing it:

```powershell
git -C agent-work/references/darktable rev-parse HEAD
git -C agent-work/references/darktable status --short

git -C agent-work/references/dnglab rev-parse HEAD
git -C agent-work/references/dnglab status --short

git -C agent-work/references/other/rawloader rev-parse HEAD
git -C agent-work/references/other/rawloader status --short
```

The commit printed by each `rev-parse` must match this manifest, and `status`
must be empty before the checkout is used as evidence. Do not update a reference
mid-task. If new evidence requires another revision, record the change and
update the owning report's inspected-version metadata.

## Exceptions

The existing `external/raw-images` submodule is not a research reference. It is
a production test-fixture dependency used by `crates/fornax-py/tests` and
`fornax-devtool`, so it remains at `external/raw-images` and in `.gitmodules`.
Moving it would change test and development behavior.

No separate downloaded DNG specification document is retained. A specification
URL or document added later must be stored under `agent-work/references/` with
its source URL, version or date, and checksum recorded here.
