# DNG Output Color Management Implementation Plan

This archived plan documents the completed DNG Output Color Management work.

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> `superpowers:executing-plans` to implement this plan task-by-task.

**Goal:** Expose the source DNG color and calibration matrices through
`ProcessMetadata` and let the postprocessor emit any of the supported
scene-linear output color spaces without changing the default `Raw` behavior.

**Architecture:** `ProcessMetadata` stores the six source DNG color tags as
fixed 3x3 `f64` arrays and camera calibrations as an identity-or-matrix enum.
`PostProcessor` selects a camera-to-XYZ matrix, composes it with the requested
output space, and applies the result in its existing row pass.

**Base branch:** `main`.

## Scope

In scope:

- `ProcessMetadata` fields for ColorMatrix1/2, ForwardMatrix1/2, and
  CameraCalibration1/2;
- `OutputColorSpace` selection for raw, sRGB, Adobe RGB, Wide Gamut RGB,
  ProPhoto RGB, XYZ, ACES2065-1, ACEScg, DCI-P3, and Rec. 2020;
- ForwardMatrix preference, ColorMatrix/calibration inversion fallback, and
  output-matrix composition;
- metadata and transform tests for all supported spaces;
- reference and public API documentation.

Out of scope:

- Matrix1/Matrix2 illuminant interpolation;
- analog-balance profile processing;
- tone curves;
- output transfer encoding in the library postprocessor;
- ICC profile generation or embedding;
- UI controls beyond the output-color enum.

## Task Checklist

- [x] Add failing tests for the six DNG source fields and output selection.
- [x] Replace the derived sRGB matrix with the six source metadata fields.
- [x] Add calibration parsing with an identity/matrix enum.
- [x] Add `OutputColorSpace` and all requested conversion matrices.
- [x] Apply forward-matrix preference and calibrated color-matrix fallback.
- [x] Add reference documentation and README guidance.
- [x] Run focused package tests.
- [x] Run full workspace validation.

## Validation Results

- Focused RED before implementation: unresolved metadata fields and
  `OutputColorSpace`.
- `cargo test -p fornax-raw-pipeline -p fornax-dng --lib`: 111 passed across
  2 suites after implementation.
- `scripts/agent-validation.sh`: passed, including workspace tests, rustdoc
  with `--no-deps`, clippy, decoder benchmark compilation, and 147
  compare-demosaic tests.

## Public Interfaces

- `ProcessMetadata<T>` gains `color_matrix_1`, `color_matrix_2`,
  `forward_matrix_1`, and `forward_matrix_2` as `Option<Matrix3x3>`, where
  `Matrix3x3 = [[f64; 3]; 3]`.
- `ProcessMetadata<T>` gains `camera_calibration_1` and
  `camera_calibration_2` as `CameraCalibration`.
- `CameraCalibration` has `Identity` and `Matrix(Matrix3x3)` variants.
- `PostProcessor` replaces `use_camera_matrix` with
  `output_color: OutputColorSpace`; its default is `OutputColorSpace::Raw`.
- The transform runs after demosaic and white balance in the existing row
  callback.

## Files Expected to Change

- `crates/fornax-raw-core/src/metadata/mod.rs`
- `crates/fornax-raw-core/src/protocol.rs`
- `crates/fornax-raw-core/src/error.rs`
- `crates/fornax-dng/src/dng_decoder/process_metadata.rs`
- `crates/fornax-dng/src/dng_decoder/mod.rs`
- `crates/fornax-dng/src/metadata.rs`
- `crates/fornax-raw-pipeline/Cargo.toml`
- `crates/fornax-raw-pipeline/src/lib.rs`
- `crates/fornax-raw-pipeline/README.md`
- `crates/fornax-decoder-rawler/src/lib.rs`
- `docs/reference/decoders/dng-output-color.md`
- `docs/architecture/overview.md`

## Validation

```bash
scripts/agent-validation.sh
```

## Decisions and Notes

- Default postprocessing remains camera RGB so existing comparison baselines
  and callers do not change.
- Matrices are stored as `f64` independently of image sample type so decoder
  metadata and postprocessor conversion do not depend on decoded precision.
- An absent camera calibration is represented as `CameraCalibration::Identity`,
  matching the DNG default.
- ForwardMatrix1 is preferred, then ForwardMatrix2. ColorMatrix1 or Matrix2 is
  multiplied by its matching calibration before inversion.
- If neither forward nor color matrices are present, output remains camera RGB
  even when a non-raw output color is requested.
- Output spaces are scene-linear. Transfer functions and ICC profile output are
  intentionally outside this PR.
