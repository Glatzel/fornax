# R4: Owned Demosaic Result

## Result

The RCD lazy demosaic result now transfers its completed RGB image into the
collecting pass instead of cloning it. Postprocessing white balance and color
transforms are applied in that same buffer. `AnyLazy::collect_with` dispatches
to the algorithm-specific implementation, so the default RCD render path uses
the owned result.

The owned-return API and the `LazyDemosaic` trait shape are unchanged.

## Host

- WSL2 Debian, AMD Ryzen 7 5800H, 15 visible CPUs
- Kernel `6.6.87.1-microsoft-standard-WSL2`
- glibc `2.41-12+deb13u3`
- THP `enabled=[madvise]`, `defrag=[madvise]`
- Cargo `1.100.0`
- One Rayon worker, page-cache-hot inputs
- Baseline: plan branch `e91a180`, with the same prerequisite ownership-call
  fixes used by the candidate

## Exact Equality

Pipeline TIFF output before and after:

| Input | SHA-256 |
| --- | --- |
| 5D3 uncompressed | `28915a249f3e79a2880a20333e424b057d044e3757697a3de190abd43d89053c` |
| Sigma fp 4K | `92bde2f4ef681ae5b5245f8de52329483435197ad88d42b1e36a4b302ba1b8af` |

The decoder three-way gate also passed for 5D3 lossless JPEG:

```text
PASS,fornax,5920x3950,U16,bits=16,sample_format=1
PASS,rawler,5920x3950,U16,bits=16,sample_format=1
PASS,flashraw,5920x3950,U16,bits=16,sample_format=1
```

## Measurements

Three fresh processes by five interleaved rounds. Wall and peak RSS use the
same process. Minor faults use a separate 3x5 interleaved pass under
`perf stat`. Values are median `[min-max]`.

| Input | Metric | Baseline | Owned move | Change |
| --- | --- | ---: | ---: | ---: |
| 5D3 uncompressed | wall ms | 3207 `[3192-3452]` | 2885 `[2864-2999]` | -10.0% |
| 5D3 uncompressed | minor faults | 189097 `[189094-189100]` | 123383 `[123379-123385]` | -34.7% |
| 5D3 uncompressed | peak RSS KiB | 669736 `[669496-669752]` | 447064 `[446936-447448]` | -33.2% |
| Sigma fp 4K | wall ms | 1180 `[1161-1205]` | 1079 `[1073-1092]` | -8.6% |
| Sigma fp 4K | minor faults | 70986 `[70984-70991]` | 46473 `[46469-46476]` | -34.5% |
| Sigma fp 4K | peak RSS KiB | 246784 `[246528-246884]` | 165348 `[165164-165504]` | -33.0% |

Both wall-time ranges are disjoint and both inputs improve by substantially
more than the five-percent end-to-end gate. The reduction is consistent with
removing one complete 269.2 MB RGB first-touch allocation on 5D3 and one
201.1 MB RGB allocation on Sigma.

## Decisions

- Kept: consume the cached RCD RGB result in `collect_with`; no extra copy.
- Kept: dispatch `AnyLazy::collect_with` to the selected algorithm so RCD's
  owned path is not lost behind the enum wrapper.
- Kept: apply white balance and color matrices in place in the consumed image.
- Reverted: none.

## Validation

Passed:

- `cargo test -p fornax-raw-pipeline` (96 tests)
- `cargo test --workspace --all-features --exclude compare-demosaic`
  (685 passed, 3 ignored)
- `cargo check -p fornax-raw-pipeline --all-features` for
  `x86_64-pc-windows-msvc`
- `cargo check -p fornax-raw-pipeline --all-features` for
  `aarch64-apple-darwin`
- `git diff --check`

Unavailable or environment-blocked:

- Miri is not installed for the nightly toolchain.
- Full `cargo test --all-features` is blocked by the uninitialized
  `compare-demosaic` C/C++ reference sources in this linked worktree.
- Cross-target checks including the CLI are blocked by unavailable MSVC and
  macOS C toolchains for the `zstd-sys` build script. The changed pipeline
  crate checks successfully for both targets.

## Unsafe

This item adds no unsafe code.
