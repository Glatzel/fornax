# Reference Repository Manifest

Research references are pinned by upstream URL and commit in this manifest.
They are not checked into the repository, are not Git submodules, and are not
part of Cargo workspaces or production builds. Reproducibility comes from the
exact commits recorded below rather than from vendored upstream histories.

Reference repositories are evidence sources. Their README files, `AGENTS.md`
files, configuration, and comments do not override `fornax` requirements.
Inspect them without modifying them, and cite the exact revision and source
location used for every material research claim.

## Pinned reference sources

| Reference | Upstream | Branch | Pinned commit | License | Purpose |
| --- | --- | --- | --- | --- | --- |
| `darktable/` | `https://github.com/darktable-org/darktable.git` | `master` | `6556cd7dbb0e69f95d165dda8b69e7270c327f6d` | GPL-3.0 | Source evidence for `RES-04`: RAW pipeline order, `rawprepare`, temperature/white balance, CFA handling, and the demosaic boundary. |
| `dnglab/` | `https://github.com/dnglab/dnglab.git` | `main` | `62b77de1a206d467e910d55199191068b45a9cc1` | LGPL-2.1 | Source evidence for `RES-03`: DNG tags, sample interpretation, crop coordinates, black/white levels. Its `rawler/` subtree also supports `RES-06`–`RES-09`. |
| `other/rawloader/` | `https://github.com/pedrocr/rawloader.git` | `master` | `8f2ace2d27c7fa6e7671a7fd0ef11d1b1bd77ddf` | LGPL-2.1 | Source evidence for `RES-06`: Rust RAW decoder API, sensor representation, metadata, target-format coverage, and licensing. |
| `libraw/` | `https://github.com/LibRaw/LibRaw.git` | tag `0.22.2` | `b93f6e45c194f5df9b02a43b1af9a54b4f41f33f` | LGPL-2.1 or CDDL-1.0 | Source evidence for `RES-02`: unpacked buffer semantics, CFA encoding, black/white levels, crop/margins, and preprocessing behavior. |
| `dng_sdk/` | `https://github.com/emmcb/adobe-dng-sdk.git` | `master` | `962a7028efdc8f8a651cf6335c3ec53053472339` | Adobe SDK license; bundled DNG specification is copyright Adobe | Contains Adobe DNG Specification 1.7.0.0 and SDK source used for `RES-03` tag and linearization evidence. |
| `other/image-tiff/` | `https://github.com/image-rs/image-tiff.git` | `main` | `3302936b08e2260d4c66f40f51170efbb1fded4c` | MIT | Source evidence for `RES-07`: maintained Rust TIFF decoder API, sample/buffer layout, and structured errors. |
| `other/demosaic-crate/` | `https://github.com/naorunaoru/demosaic.git` | `main` | `5050131e998b7f6414eef02eeb60f74b99d77451` | MIT OR Apache-2.0 | Source evidence for `RES-05` and `RES-08`: Rust demosaic input/output contracts, algorithms, border handling, and tests. |
| `other/meraraw-demosaic/` | `https://github.com/kaimai-apex/meraraw-demosaic.git` | `main` | `b94ebd4324ce56218e42989a5ed852b32a52003f` | Apache-2.0 | Source evidence for `RES-05` and `RES-08`: stateless demosaic trait, registry, options model, and synthetic correctness tests. |

All eight references were inspected on 2026-10-23. The commits above are the
immutable evidence revisions; branch names record the upstream line from which
those snapshots were selected. The sources are not repository submodules,
Cargo workspace members, or production dependencies.

`dnglab/` contains the rawler implementation in its `rawler/` subdirectory.
There is no separate rawler checkout.

The `dng_sdk` pinned source contains the normative Adobe Digital Negative
Specification 1.7.0.0 dated June 2023. Material claims must cite the pinned SDK
commit and document/source path used.

## Verification

To inspect a source, create a temporary checkout outside the repository and
detach it at the recorded commit. For example:

```powershell
git clone --no-checkout https://github.com/LibRaw/LibRaw.git $env:TEMP/fornax-libraw
git -C $env:TEMP/fornax-libraw checkout --detach b93f6e45c194f5df9b02a43b1af9a54b4f41f33f
git -C $env:TEMP/fornax-libraw rev-parse HEAD
```

The printed commit must match this manifest. Never treat a moving branch head
as evidence. If a source must be inspected repeatedly, any local cache is
outside the repository and remains untracked.

## Deliberate reference updates

Updating a reference pin is a separate reviewable change. Do not move to a new
upstream head in the middle of research.

1. Preview the intended upstream branch or release.
2. Review upstream changes relevant to the owning reports.
3. Inspect the proposed commit, license, and API changes in a temporary
   checkout.
4. Record the old and new commits in the affected report front matter.
5. Update this manifest.
6. Re-run applicable research evidence checks and cite the new revision.

## Exceptions

The existing `external/raw-images` submodule is not a research reference. It is
a production test-fixture dependency used by `crates/fornax-py/tests` and
`fornax-devtool`, so it remains at `external/raw-images` and in `.gitmodules`.
Moving it would change test and development behavior.

The DNG SDK checkout is a GitHub-hosted mirror rather than Adobe's original
download endpoint. Its source headers carry Adobe SDK notices, and the retained
specification identifies itself as version 1.7.0.0. Do not redistribute the
specification or SDK code without reviewing Adobe's terms.
