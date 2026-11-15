# Workspace refactor baseline

This archived workspace-refactor record documents Workspace refactor baseline.

## Purpose

This note records the pre-refactor measurements, golden decode cases, and
verification of the advisory feasibility analysis. The refactor scope accepted
by the maintainer is steps 1 through 6 only: record baselines, merge the
pre/post processors, move generic tag accessors, add a codec sample buffer,
move TIFF raster handling, and move DNG-owned constants. Format/facade
consolidation, workspace splitting, and publication are out of scope.

## Baseline

- Baseline commit: `9b9c1b284ff5f4fe9ef1c8ff41eb6c3cff17b8d2`.
- Workspace test run: `cargo test --workspace --all-features --exclude compare-demosaic`.
- Result: 251 tests passed across 35 suites; Cargo reported 5.97 seconds of
  test execution and 58.78 seconds wall time from a fresh target directory.
- Clean release CLI build: 68.84 seconds, 171 crates compiled.
- Release `fornax` binary: 15,297,048 bytes.
- Release `decoder-tracker` binary: 11,365,392 bytes after an incremental
  release build of 17.28 seconds.
- No-op release rebuild after the clean CLI build: 0.26 seconds.
- Normal dependency-tree line count: `fornax-cli` 385, `fornax-tiff` 27,
  `fornax-codec` 151.

These timings are single-host observations, not portable thresholds. Compare
later measurements on the same machine and target cache conditions.

## Golden decode set

The selected manifest contains nine pointers totaling 337,082,612 bytes
(321.47 MiB):

- Canon EOS 5D Mark III DNG regenerated as uncompressed and lossless JPEG;
- Canon EOS 5D Mark III fp16 and fp32 Deflate DNGs with predictors;
- Apple iPhone SE BGGR DNG;
- Google Pixel 8 Pro DNG;
- Sony ILCE-9 compressed and uncompressed ARW;
- Sony ILCE-1M2 lossless-compressed ARW.

The hash gate covers storage-native samples, cropped `f32` samples, normalized
process metadata, and the merged TIFF/DNG-style metadata directory. The
compressed and uncompressed Canon DNG pair intentionally has matching sample
and process hashes while retaining different DNG metadata hashes. Compressed
and uncompressed Sony ARW cases retain distinct sample hashes.

The committed corpus gitlink `a96f50803e8c7b2923badc983cba80246b80eb1e` was no
longer fetchable because raw.pixls.us replaces its daily snapshot history. The
current pointer snapshot was used only after `manifest --check` confirmed it
matches the committed 2020-row manifest. The submodule pointer is not part of
the baseline PR.

## Advisory verification

| Advisory claim or recommendation | Verification |
| --- | --- |
| Core split can be skipped by removing codec's dependency on core | Feasible: codec is the only base-candidate crate with a normal core dependency, but `fornax-devtool` must still be assigned a workspace or remain raw-side. |
| Ten crates depend on core | Confirmed with `cargo metadata`: 10 direct normal/dev dependents. |
| Roughly 54 files import core | The proposal baseline measured 54 source/test/bench files; the exact query varies with aliases and facade imports. |
| `RawMosaic` affects 27 files and 234 occurrences | Confirmed at the proposal baseline. New baseline-tool references change current totals and must not be used as the historical count. |
| A wrapper can keep `RawMosaic` call sites stable | Plausible and worth verifying in step 4. Current generic codec callers are DNG decode and DNG writer paths; ARW uses vendor codecs directly. |
| Only codec call sites need to change | Incomplete. Codec tests, devtool byte accounting, and the tracker consume `RawMosaic`; an adapter should keep them source-compatible, but those call sites remain validation surface. |
| `active_area` and `default_crop` are DNG tag policy | Confirmed for DNG. ARW has different Sony tag and model-fallback semantics, so they are not one generic TIFF helper. |
| `crop_and_cast` interprets DNG tags | Incorrect. It crops a `Rect` and casts samples; it contains no DNG tag interpretation. |
| Pre/post merge is mechanical and low risk | Confirmed dependencies and ownership are simple, but it still moves 8,496 source lines and 89 tests plus public paths. The maintainer correctly chose it as the first code step. |
| Path plus version avoids `[patch]` | Confirmed by Cargo documentation: path is used locally and registry version is used when published. |
| `Primitive: Float` is inappropriate for a neutral integer raster | Confirmed at `crates/fornax-raw-core/src/primitive.rs:12`. Neutral buffer shape remains OPEN. |
| TIFF raster should receive resolved codec compression | Sound design recommendation, not implemented or decided in this PR. |
| DNG compression 34892 is DNG lossy JPEG | Confirmed by DNG 1.4/1.7 material. |
| DNG compression 52546 is DNG 1.7 JPEG XL | Confirmed by DNG 1.7 and LibTIFF. |
| Compression 50000 is Zstandard but not Adobe-registered | Confirmed by LibTIFF 4.0.10 release notes. |
| Compression 50002 is JPEG XL but not DNG-defined | Confirmed by GDAL maintainer discussion; it was a self-assigned TIFF value later superseded by 52546 on write. |
| CR3 is BMFF with TIFF-structured metadata | Consistent with the existing CR3 plan (`docs/decoder/formats/canon-cr3.md`); no CR3 implementation exists to test. |
| Vendor codecs should wait for a second vendor | Consistent with the accepted scope, which does not move vendor codecs. |
| No compatibility shim is needed | Maintainer-confirmed because Fornax is not published on crates.io. |
| DNC belongs in a platform-gated raw facade module | Maintainer decision. It is not implemented in this PR. |
| Rawler wrapper remains dev-only | Confirmed at `crates/fornax-decoder/benches/rawler_wrapper.rs:11`. |

## Sources

- Adobe, *Digital Negative (DNG) Specification 1.7.1.0*, September 2023:
  <https://helpx.adobe.com/content/dam/help/en/camera-raw/digital-negative/jcr_content/root/content/flex/items/position/position-par/download_section_733958301/download-1/DNG_Spec_1_7_1_0.pdf>.
- LibTIFF, *Changes in TIFF v4.0.10*:
  <https://libtiff.gitlab.io/libtiff/releases/v4.0.10.html>.
- GDAL maintainer thread on TIFF JPEG XL identifiers:
  <https://www.mail-archive.com/gdal-dev@lists.osgeo.org/msg42055.html>.
- Cargo Book, dependency locations:
  <https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#multiple-locations>.

## Open decisions

- Neutral buffer shape remains undecided.
- Vendor codec placement remains undecided and is outside this scope.
- Crate names remain undecided.
- Compression-ID ownership is recorded but is not changed in this PR.
