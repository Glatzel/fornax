# RAW corpus and regression tracking

## Decision status

The acquisition choice is not approved yet. The recommendation in this
document is to extend `scripts/get_raw_samples.py`; do not add a corpus
submodule until the repository owner approves that choice.

No full corpus was downloaded while preparing this design.

## Findings

`raw.pixls.us` exposes a searchable repository and three full-data access
forms:

- HTTPS under `/data/`;
- a Git LFS mirror at `https://raw.pixls.us/data.lfs.git`;
- a Git-annex mirror at `https://raw.pixls.us/data.annex.git`.

The corpus is binary RAW data. Adding either Git mirror as a normal submodule
would couple every checkout or CI preparation step to Git LFS or git-annex,
and still fetches the same RAW payloads. It does not satisfy the repository
rule against committing binary files, adds a large external dependency, and is
unnecessary because the site already provides a checksum-bearing listing.

The existing `scripts/get_raw_samples.py` already:

- downloads the repository listing from the site's JSON API and caches its
  ETag;
- uses DuckDB to extract URL, SHA-256, and added date from the listing;
- filters by case-insensitive filename wildcards;
- builds aria2 input with one connection per file and a SHA-256 checksum;
- writes under `external/raw.pixls.us` by default, where `external/` is
  ignored.

The Pixi `data` environment already declares `python-duckdb`, `httpx`, `typer`,
and `aria2`. No global tool installation is needed.

The site states that newer samples are CC0 and that some older samples retain
more restrictive licences. The acquisition tool must retain the listing's
licence metadata rather than assuming every sample is CC0.

## Acquisition options

| Option | Advantages | Costs and risks | Recommendation |
| --- | --- | --- | --- |
| Add raw.pixls.us as a submodule | One pinned checkout; familiar Git tracking. | Binary corpus, very large checkout, Git LFS/annex requirement, platform-specific filenames, poor CI incrementality, conflicts with the no-binaries policy. | Reject by default. |
| Extend `scripts/get_raw_samples.py` | Uses the existing checksum-verified path; downloads only selected cameras/formats; works on Linux CI and Windows locals; no binary history in Fornax. | The script needs format/camera selection and manifest export; upstream listing changes are an external dependency. | Recommended, pending owner approval. |

Recommended extension:

1. Keep the existing JSON cache, DuckDB extraction, and aria2 checksum path.
2. Add explicit `--format` and `--camera` filters alongside filename
   wildcards.
3. Emit a canonical manifest with vendor, camera name, model, format,
   checksum, original filename, relative path, URL, added date, and licence.
4. Make `--list` and `--manifest-only` mutually usable so CI can plan a shard
   without downloading its files.
5. Add deterministic `--shard INDEX/COUNT` selection after sorting by SHA-256.
6. Default downloads to an ignored directory and fail if `--out` resolves
   inside a tracked repository path.
7. Never write RAW, DNG, preview, or image files into the Git worktree except
   through an explicitly ignored output directory.

The corpus baseline does not depend on a submodule. The committed data is a
small CSV baseline plus metadata-only manifest.

## Tracking CLI

Follow the `compare-demosaic` architecture:

- a small workspace crate, provisionally `compare-decoder`;
- subcommands `run`, `diff`, and optional `readme`;
- local filters `--format`, `--camera`, and `--filename`;
- deterministic `--shard INDEX/COUNT`;
- a committed baseline CSV;
- generated output under `target/compare-decoder/`;
- output written only to an explicit ignored path;
- the same binary and filters used locally and in CI.

The CLI must not contain a separate ad-hoc corpus traversal path. Local
debugging runs through `compare-decoder run`, normally with one camera or
filename filter.

## Baseline schema

The minimum required columns are:

```csv
vendor,camera,model,format,filename,pass,error_kind,width,height,cfa
```

Definitions:

| Column | Definition |
| --- | --- |
| `vendor` | The listing make/manufacturer. |
| `camera` | Stable display name, normally `<vendor> <model>`. |
| `model` | The listing model, without the vendor prefix. |
| `format` | Uppercase source format: `DNG`, `CR3`, `ARW`, `NEF`, `RAF`, `3FR`, `RW2`, or `IIQ`. |
| `filename` | Original filename without its extension. When the downloaded file is checksum-named, record the source stem and keep the SHA in the manifest. |
| `pass` | `1` or `0`, exactly as defined below. |
| `error_kind` | Stable `ErrorKind` value, or empty on success. |
| `width`, `height` | Full decoded dimensions; `0` only when decoding failed before geometry was available. |
| `cfa` | Canonical CFA description, for example `RGGB`, `XTrans-6x6`, or `unknown`. |

These extra columns are justified because they make failures actionable
without a local RAW file: `error_kind` identifies unsupported camera versus
variant/corruption/handoff, dimensions catch geometry drift, and `cfa`
distinguishes a decode success from a successful Bayer handoff.

Do not include decode time, host, commit, timestamp, or noisy floating-point
metrics in the committed baseline. Those values are volatile and make
regression diffs meaningless.

## Proposed `pass` definition

`pass = 1` if and only if all applicable conditions hold:

1. The native decoder returns `Ok(DecodedRaw)` and the process completes
   within the corpus runner's fixed timeout.
2. Metadata and geometry are sane:
   - dimensions are non-zero and equal to the sample count;
   - the CFA repeat dimensions are non-zero and match the pattern length;
   - every CFA plane index is valid for `CFAPlaneColor`;
   - `ActiveArea` is within the full image;
   - `DefaultCropOrigin` plus `DefaultCropSize` is within `ActiveArea`;
   - black levels are finite and below the corresponding white level;
   - white levels are positive;
   - required identity/camera and orientation values are well-typed.
3. When a reviewed external reference result is available, pixel agreement is
   checked after aligning the full image, active area, CFA phase, endianness,
   and sample interpretation. The comparison must be exact for stored integer
   samples. Without a comparable reference, the condition is recorded as
   `reference=unavailable` and treated as not applicable, not as a failure.
4. The native 2x2 Bayer handoff succeeds for a conventional Bayer input. For
   any non-2x2 input, the handoff instead must return
   `HandoffError::UnsupportedCfa`; that typed rejection is a pass for corpus
   purposes and must not be recorded as a decode failure.

`pass = 0` on any decode error, metadata sanity failure, comparable pixel
mismatch, missing required output field, or untyped/unexpected panic. An
explicit unsupported-camera row is still a valid baseline row with `pass=0`,
`error_kind=unsupported_camera`; it is not silently omitted.

The `pass` definition needs owner approval before the first committed
baseline.

## CI design

Full corpus runs happen in CI only. Local runs are filtered investigations.

The CI job is a matrix over format or tier shards:

1. Fetch the cached listing and generate the metadata manifest.
2. Select the shard deterministically by SHA-256.
3. Download only that shard's files via the same
   `scripts/get_raw_samples.py` path used locally.
4. Run `compare-decoder run` with the shard and compare its rows with the
   matching committed baseline rows.
5. Fail on any missing, new, or changed row, with `pass` changes as
   regressions.
6. Upload `target/compare-decoder/` and the text diff only when the comparison
   finds a difference.
7. Do not cache or upload RAW files as long-lived artifacts. A failed job may
   retain them only under the CI workspace's normal ephemeral retention if
   debugging requires it; the uploaded diagnostic is the CSV/diff, not the RAW.

Each format shard may run multiple camera jobs. Camera rows are never
collapsed into a format-only result because cameras sharing a container can
use different compression, CFA, and metadata variants.

The first baseline is generated from a CI run and reviewed before commit.
Local numeric differences are not accepted as baseline edits without evidence
and review.

## Open approval requests

- Approve extending `scripts/get_raw_samples.py` instead of adding a
  raw.pixls.us submodule.
- Approve the `pass` definition above.
- Confirm whether the committed manifest should include the source licence
  column and whether non-CC0 samples may be used in CI.
- Confirm whether `compare-decoder` should be its own crate or folded into the
  existing devtool/benchmark tooling after the scaffold lands.
