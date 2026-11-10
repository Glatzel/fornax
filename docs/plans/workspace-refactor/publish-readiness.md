# Publish Readiness Report

## Scope and method

This is a read-only snapshot of the Cargo workspace on 2026-11-10. It
inventories publishability, checks package metadata and path dependencies,
queries crate-name availability, inspects package contents and archive sizes,
runs Cargo publish dry runs in dependency order, and records docs.rs feature
behavior. It does not publish, reserve names, edit manifests or versions, or
recommend whether or when to publish.

The workspace version is `0.0.17`. Cargo's semver rules permit breaking changes
between `0.0.x` patch releases, so the version string alone does not promise
patch-level compatibility.

## Workspace inventory

`cargo metadata --no-deps --format-version 1` reported 12 workspace members:

| Crate | Version | Publishability |
| --- | --- | --- |
| `compare-demosaic` | 0.0.17 | `publish = false` |
| `decoder-tracker` | 0.0.17 | `publish = false` |
| `fornax-arw` | 0.0.17 | Publishable per Cargo |
| `fornax-cli` | 0.0.17 | Publishable per Cargo |
| `fornax-codec` | 0.0.17 | Publishable per Cargo |
| `fornax-decoder` | 0.0.17 | Publishable per Cargo |
| `fornax-devtool` | 0.0.17 | Publishable per Cargo |
| `fornax-dng` | 0.0.17 | Publishable per Cargo |
| `fornax-raw` | 0.0.17 | Publishable per Cargo |
| `fornax-raw-core` | 0.0.17 | Publishable per Cargo |
| `fornax-raw-pipeline` | 0.0.17 | Publishable per Cargo |
| `fornax-tiff` | 0.0.17 | Publishable per Cargo |

`fornax-py` is not a workspace member. The root manifest explicitly excludes
`./crates/fornax-py/` while its API is unstable.

## Metadata completeness

Every publishable crate sets `readme = "README.md"`, has that file on disk, and
inherits version `0.0.17`. The following fields are absent from every
publishable package:

- `description`
- `license`
- `repository`
- `rust-version`

Only `fornax-tiff` has `package.metadata.docs.rs`; it sets
`all-features = true`. The other nine publishable crates have no docs.rs
metadata.

| Crate | Description | License | Repository | README | rust-version | Version | docs.rs metadata |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `fornax-arw` | Missing | Missing | Missing | Present | Missing | 0.0.17 | Missing |
| `fornax-cli` | Missing | Missing | Missing | Present | Missing | 0.0.17 | Missing |
| `fornax-codec` | Missing | Missing | Missing | Present | Missing | 0.0.17 | Missing |
| `fornax-decoder` | Missing | Missing | Missing | Present | Missing | 0.0.17 | Missing |
| `fornax-devtool` | Missing | Missing | Missing | Present | Missing | 0.0.17 | Missing |
| `fornax-dng` | Missing | Missing | Missing | Present | Missing | 0.0.17 | Missing |
| `fornax-raw` | Missing | Missing | Missing | Present | Missing | 0.0.17 | Missing |
| `fornax-raw-core` | Missing | Missing | Missing | Present | Missing | 0.0.17 | Missing |
| `fornax-raw-pipeline` | Missing | Missing | Missing | Present | Missing | 0.0.17 | Missing |
| `fornax-tiff` | Missing | Missing | Missing | Present | Missing | 0.0.17 | `all-features = true` |

These omissions are publish blockers; this report does not change them.

## Path dependencies

All 17 normal, non-development path dependencies in publishable crates are
path-only entries: Cargo reports a wildcard `*` requirement and no registry
version requirement. No publishable crate has a non-development build
dependency with a path source. None currently combines a path with a published
registry version.

| Publishable crate | Path dependency | Requirement |
| --- | --- | --- |
| `fornax-arw` | `fornax-dng`, `fornax-raw-core`, `fornax-tiff` | `*` |
| `fornax-cli` | `fornax-dng`, `fornax-raw` | `*` |
| `fornax-codec` | none | n/a |
| `fornax-decoder` | `fornax-arw`, `fornax-dng`, `fornax-raw-core` | `*` |
| `fornax-devtool` | `fornax-raw-core` | `*` |
| `fornax-dng` | `fornax-codec`, `fornax-raw-core`, `fornax-tiff` | `*` |
| `fornax-raw` | `fornax-decoder`, `fornax-raw-core`, `fornax-raw-pipeline` | `*` |
| `fornax-raw-core` | none | n/a |
| `fornax-raw-pipeline` | `fornax-raw-core` | `*` |
| `fornax-tiff` | `fornax-codec` | `*` |

Every path-only workspace dependency is a publish blocker for the dependent
crate. Development-only path dependencies are outside this normal/build audit.

## Crate-name availability

Queried crates.io on 2026-11-10 at 11:30 UTC. `404` meant no crate endpoint was
found at query time; it is not a name reservation.

| Crate | HTTP status | Result |
| --- | --- | --- |
| `fornax-arw` | 404 | Available at query time |
| `fornax-cli` | 404 | Available at query time |
| `fornax-codec` | 404 | Available at query time |
| `fornax-decoder` | 404 | Available at query time |
| `fornax-devtool` | 404 | Available at query time |
| `fornax-dng` | 404 | Available at query time |
| `fornax-raw` | 404 | Available at query time |
| `fornax-raw-core` | 404 | Available at query time |
| `fornax-raw-pipeline` | 404 | Available at query time |
| `fornax-tiff` | 404 | Available at query time |

Query command:

```bash
for crate in fornax-raw-core fornax-codec fornax-tiff fornax-dng fornax-arw \
  fornax-raw-pipeline fornax-devtool fornax-decoder fornax-raw fornax-cli; do
  curl -sS -o /dev/null -w '%{http_code}\n' \
    --user-agent 'fornax-publish-readiness-audit (https://github.com/Glatzel/fornax)' \
    "https://crates.io/api/v1/crates/${crate}"
done
```

## Package contents and sizes

Implementation in progress.

## Publish dry runs

Implementation in progress.

## docs.rs feature sets

Implementation in progress.

## Blockers

- All 10 publishable crates omit `description`.
- All 10 publishable crates omit `license`.
- All 10 publishable crates omit `repository`.
- All 10 publishable crates omit `rust-version`.
- Every crate with a normal workspace dependency uses path-only entries.

## `[UNVERIFIED]` items

None from the inventory and name-availability phase.
