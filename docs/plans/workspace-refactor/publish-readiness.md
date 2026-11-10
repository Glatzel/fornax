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

### Command record

```bash
cargo metadata --no-deps --format-version 1 > /tmp/fornax-publish-metadata.json
jq -r '.packages[] | [.name, .version, (.publish | tostring), .manifest_path] | @tsv' \
  /tmp/fornax-publish-metadata.json
jq -r '.packages[] | select(.publish == null) |
  [.name, .version, (.description // "<missing>"), (.license // "<missing>"),
   (.repository // "<missing>"), (.readme // "<missing>"),
   (.rust_version // "<missing>"), (.metadata["docs.rs"] // "<missing>")]
  | @tsv' /tmp/fornax-publish-metadata.json
jq -r '.packages[] | select(.publish == null) | .name as $package |
  .dependencies[] |
  select((.kind == null or .kind == "build") and .path != null) |
  [$package, .name, .req, .path, (.kind // "normal")] | @tsv' \
  /tmp/fornax-publish-metadata.json
```

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

The same manifest checks also expose 16 normal, non-development Git
dependencies with wildcard `*` requirements. These are additional publish
blockers because Cargo requires a registry version requirement before it can
replace a Git source in a published manifest:

| Publishable crate | Git dependencies without versions |
| --- | --- |
| `fornax-arw` | `generic-num`, `image` |
| `fornax-cli` | `clap-style`, `clerk`, `image`, `mischief` |
| `fornax-codec` | `image` |
| `fornax-decoder` | `clerk` |
| `fornax-devtool` | `clerk`, `generic-num`, `image`, `mischief` |
| `fornax-dng` | `generic-num`, `image` |
| `fornax-raw` | none |
| `fornax-raw-core` | `image` |
| `fornax-raw-pipeline` | `generic-num` |
| `fornax-tiff` | none |

## Crate-name availability

Queried crates.io on 2026-11-10 at 11:30 UTC. `404` meant no crate endpoint was
found at query time; it is not a name reservation.
The initial UA-less requests were rejected with `403`; the recorded results use
the descriptive user agent shown in the command below and completed normally.

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

`cargo package --list -p <crate>` completed for all 10 crates. The lists total
143 planned archive entries. Three entries in every list are Cargo-generated
archive files (`.cargo_vcs_info.json`, `Cargo.lock`, and `Cargo.toml.orig`);
the remaining 113 entries exist in the worktree. Their measured size is
920,638 bytes.

| Crate | Listed entries | On-disk listed bytes |
| --- | ---: | ---: |
| `fornax-arw` | 11 | 60,311 |
| `fornax-cli` | 11 | 35,053 |
| `fornax-codec` | 23 | 161,848 |
| `fornax-decoder` | 9 | 26,716 |
| `fornax-devtool` | 8 | 14,093 |
| `fornax-dng` | 17 | 126,701 |
| `fornax-raw` | 10 | 1,820 |
| `fornax-raw-core` | 13 | 42,606 |
| `fornax-raw-pipeline` | 24 | 347,881 |
| `fornax-tiff` | 17 | 103,609 |

No list contains `external/`, `temp/`, raw image extensions, generated image
samples, fixture trees, or local-only development assets. Benchmark source
files are included in `fornax-decoder`, `fornax-dng`, and
`fornax-raw-pipeline`; they are source, not local data assets. The repository
root `LICENSE` is not listed in any package.

Command:

```bash
cargo package --list -p <crate> > /tmp/fornax-<crate>-package-files.txt
```

The on-disk bytes were measured by reading each list path relative to
`crates/<crate>/`, calling `stat -c %s` for existing files, and summing those
file sizes. The three Cargo-generated archive entries in each list do not exist
before successful archive creation.

`cargo package --no-verify -p <crate>` failed for every crate with exit code
101 before creating an archive. No `.crate` archive or full uncompressed
package directory was produced, so archive sizes and full package totals are
not available from this workspace snapshot.

| Crate | Archive result | First actionable error |
| --- | --- | --- |
| `fornax-raw-core` | Failed, exit 101 | `image` has no version requirement |
| `fornax-codec` | Failed, exit 101 | `image` has no version requirement |
| `fornax-tiff` | Failed, exit 101 | `fornax-codec` has no version requirement |
| `fornax-dng` | Failed, exit 101 | `fornax-codec` has no version requirement |
| `fornax-arw` | Failed, exit 101 | `fornax-dng` has no version requirement |
| `fornax-raw-pipeline` | Failed, exit 101 | `fornax-raw-core` has no version requirement |
| `fornax-devtool` | Failed, exit 101 | `clerk` has no version requirement |
| `fornax-decoder` | Failed, exit 101 | `clerk` has no version requirement |
| `fornax-raw` | Failed, exit 101 | `fornax-decoder` has no version requirement |
| `fornax-cli` | Failed, exit 101 | `clap-style` has no version requirement |

Command:

```bash
for crate in fornax-raw-core fornax-codec fornax-tiff fornax-dng fornax-arw \
  fornax-raw-pipeline fornax-devtool fornax-decoder fornax-raw fornax-cli; do
  cargo package --no-verify -p "$crate"
done
```

## Publish dry runs

`cargo publish --dry-run -p <crate>` failed for every crate with exit code 101.
The first actionable error is shown below. Unpublished path dependencies and
versionless Git dependencies are recorded as evidence and were not worked
around.

| Order | Crate | Result | First actionable error |
| ---: | --- | --- | --- |
| 1 | `fornax-raw-core` | Fail | `image` has no version requirement |
| 2 | `fornax-codec` | Fail | `image` has no version requirement |
| 3 | `fornax-tiff` | Fail | `fornax-codec` has no version requirement |
| 4 | `fornax-dng` | Fail | `fornax-codec` has no version requirement |
| 5 | `fornax-arw` | Fail | `fornax-dng` has no version requirement |
| 6 | `fornax-raw-pipeline` | Fail | `fornax-raw-core` has no version requirement |
| 7 | `fornax-devtool` | Fail | `clerk` has no version requirement |
| 8 | `fornax-decoder` | Fail | `clerk` has no version requirement |
| 9 | `fornax-raw` | Fail | `fornax-decoder` has no version requirement |
| 10 | `fornax-cli` | Fail | `clap-style` has no version requirement |

Command:

```bash
for crate in fornax-raw-core fornax-codec fornax-tiff fornax-dng fornax-arw \
  fornax-raw-pipeline fornax-devtool fornax-decoder fornax-raw fornax-cli; do
  cargo publish --dry-run -p "$crate"
done
```

## docs.rs feature sets

According to docs.rs, absent `[package.metadata.docs.rs]` means
`features = []` (default), `all-features = false` (default), and
`no-default-features = false` (default). Cargo therefore builds the crate's
default feature set. `all-features = true` passes `--all-features`. Source:
<https://docs.rs/about/metadata>.

| Crate | Declared `[features]` | docs.rs metadata | Resulting docs.rs feature set |
| --- | --- | --- | --- |
| `fornax-arw` | none | absent | Cargo defaults; no crate features |
| `fornax-cli` | none | absent | Cargo defaults; no crate features |
| `fornax-codec` | `default = [lzw, packbits]`; `deflate`, `jpegxl`, `lossless-jpeg`, `lossy-jpeg`, `lzw`, `packbits`, `zstd` | absent | Default: `lzw`, `packbits` |
| `fornax-decoder` | `zstd` | absent | Default: no crate features |
| `fornax-devtool` | none | absent | Cargo defaults; no crate features |
| `fornax-dng` | `default = []`; `metadata-json`, `zstd` | absent | Default: no crate features |
| `fornax-raw` | `zstd` | absent | Default: no crate features |
| `fornax-raw-core` | none | absent | Cargo defaults; no crate features |
| `fornax-raw-pipeline` | none | absent | Cargo defaults; no crate features |
| `fornax-tiff` | `default = [lzw, packbits]`; `jpegxl`, `lzw`, `packbits`, `raster`, `zstd` | `all-features = true` | All declared features |

## Validation

- `scripts/agent-validation.sh --docs-only`: pass.
- `git diff --check`: pass.
- `cargo tree -p fornax-codec -e normal --depth 1`: pass; direct dependencies
  are `bytemuck`, `image`, `rayon`, `thiserror`, and `weezl`.
- `cargo tree -p fornax-tiff -e normal --depth 1`: pass; direct dependencies
  are `fornax-codec`, `num_enum`, and `strum`.
- Golden hash command: stopped during local LFS materialization because
  `external/raw.pixls.us/Sony/ILCE-9/Sony_-_ILCE-9_-_uncompressed.ARW` was not
  created. CI remains authoritative for this LFS-backed check.

The dependency-tree outputs were:

```text
fornax-codec v0.0.17
├── bytemuck v1.25.2
├── image v0.25.9 (https://github.com/Glatzel/image-rs#d11aaf5f)
├── rayon v1.12.0
├── thiserror v2.0.21
└── weezl v0.1.12

fornax-tiff v0.0.17
├── fornax-codec v0.0.17
├── num_enum v0.7.6
└── strum v0.28.0
```

## Blockers

- All 10 publishable crates omit `description`.
- All 10 publishable crates omit `license`.
- All 10 publishable crates omit `repository`.
- All 10 publishable crates omit `rust-version`.
- All 10 package operations fail before archive creation.
- All 10 publish dry runs fail.
- Every crate with a normal workspace dependency uses path-only entries; none
  also specifies a registry version.
- Sixteen normal Git dependencies use wildcard requirements without a registry
  version.
- The repository root `LICENSE` is not included in the planned package lists.

## Unresolved decisions

- Crate names, versioning, workspace splitting, and whether or when to publish
  remain maintainer-owned. This report makes no decision on any of them.
- The maintainer states that Fornax has never been published and that there is
  no publication plan; this report is evidence only and does not create one.

## `[UNVERIFIED]` items

- `[UNVERIFIED]` Full uncompressed package totals and archive sizes: package
  operations stopped during manifest validation, so these cannot be observed.
  The source-file measurements above are exact but are not complete package
  totals.
