# `compare-demosaic`: Synthetic Demosaic Quality Comparison

## Goal

Build a reproducible, pure-Rust + Python demosaic quality comparison harness in `crates/compare-demosaic` and a dedicated `compare-demosaic` pixi environment. The harness generates a 16-tile `2048x2048` synthetic SVG test chart, exports 5 linear-light `float32` TIFFs (1 RGB ground truth + 4 Bayer mosaics), runs Fornax's production-ready demosaicing methods (`linear`, `ppg`) in Rust, evaluates accuracy/chroma/unclamped-float/noise metrics in Python, updates `crates/compare-demosaic/README.md`, and runs via a `workflow_dispatch` GitHub Actions workflow.

## Scope

### In scope
- `compare-demosaic` pixi feature, tasks, and environment in `pixi.toml`
- 16-tile `2048x2048` synthetic SVG chart (`crates/compare-demosaic/chart/demosaic_chart.svg`)
- Python rasterizer (`resvg` 4x supersampling + linear-light 4x4 box downsample + analytical `f32` synthesis) exporting 5 `float32` TIFFs to `temp/compare-demosaic/`
- Rust CLI crate (`crates/compare-demosaic`) reading Bayer `f32` TIFFs via `fornax-tiff`, running `fornax_raw::demosaic::Demosaic` (`Linear`, `Ppg`, `all`), and writing demosaiced `f32` RGB TIFFs to `temp/compare-demosaic/{method}/`
- Python metrics evaluator (`MAE`, `MSE`, `RMSE`, `MaxAE`, `PSNR`, per-channel `PSNR`, `SSIM`, false-color chroma error, unclamped float & mosaic-only noise stats), CSV/JSON persistence, preview/error-heatmap PNGs, and `crates/compare-demosaic/README.md` generation
- GitHub Actions workflow `.github/workflows/compare-demosaic.yml`
- Cleanup of obsolete `compare-demosaic` exclusion/setup hook in `scripts/agent-validation.sh`

### Out of scope
- Non-production demosaicers (`AMaZE`, `RCD`, `LMMSE`, `VNG`, `VNG4`) as default targets
- Non-2x2 Bayer CFAs (X-Trans, etc.)
- Modifying `fornax-raw` demosaic algorithm implementations

## Task checklist

### Phase 1 (Chart design & review checkpoint)
- [x] Create `chore/compare-demosaic` worktree and initialize fixtures
- [x] Add `docs/plans/chore-compare-demosaic.md` and allow `crates/compare-demosaic/chart/*.svg` in `.gitignore`
- [x] Configure `compare-demosaic` environment and tasks in `pixi.toml`
- [x] Implement `chart_spec.py` and `generate_chart.py` (`svg` subcommand)
- [x] Generate `crates/compare-demosaic/chart/demosaic_chart.svg`, lint, render preview via `resvg`, and stop for user review

### Phase 2 (TIFF export, Rust runner, evaluation, CI)
- [x] Complete `generate_chart.py` `export`/`all` subcommands (5 `float32` TIFFs in `temp/compare-demosaic/`)
- [x] Implement `crates/compare-demosaic` Rust crate (`Cargo.toml`, `src/lib.rs`, `src/main.rs`, tests)
- [x] Implement `evaluate.py`, `run.sh`, `test_compare_demosaic.py`, and generate `crates/compare-demosaic/README.md` + `data/`
- [x] Add ISO 12233-style slanted-edge MTF analysis (`mtf.py`) and SVG MTF charts (`mtf_comparison.svg` + `{method}/mtf_{pattern}.svg`)
- [x] Clean up `scripts/agent-validation.sh` and `scripts/agent-validation-test.sh`
- [x] Add `.github/workflows/compare-demosaic.yml`
- [x] Run `pixi run -e compare-demosaic compare-demosaic-test` and `scripts/agent-validation.sh`

## Files expected to change

- `.gitignore`
- `pixi.toml`
- `docs/plans/chore-compare-demosaic.md`
- `crates/compare-demosaic/Cargo.toml`
- `crates/compare-demosaic/README.md`
- `crates/compare-demosaic/chart/demosaic_chart.svg`
- `crates/compare-demosaic/data/results.csv`
- `crates/compare-demosaic/data/tile_results.csv`
- `crates/compare-demosaic/data/results.json`
- `crates/compare-demosaic/scripts/chart_spec.py`
- `crates/compare-demosaic/scripts/generate_chart.py`
- `crates/compare-demosaic/scripts/evaluate.py`
- `crates/compare-demosaic/scripts/run.sh`
- `crates/compare-demosaic/scripts/test_compare_demosaic.py`
- `crates/compare-demosaic/src/lib.rs`
- `crates/compare-demosaic/src/main.rs`
- `scripts/agent-validation.sh`
- `scripts/agent-validation-test.sh`
- `.github/workflows/compare-demosaic.yml`

## Validation

- Phase 1: `pixi run -e compare-demosaic chart-svg`, `pixi run -e compare-demosaic compare-demosaic-lint`, `resvg` preview inspection
- Phase 2: `pixi run -e compare-demosaic compare-demosaic --method all`, `pixi run -e compare-demosaic compare-demosaic-test`, `scripts/agent-validation.sh`

## Decisions and notes

- **Layout:** 4x4 grid (`2048x2048`), `512x512` tiles, `448x448` even-aligned ROIs (`ROI_PAD_X=32`, `ROI_PAD_TOP=48`) so every tile aligns identically to the 2x2 Bayer grid.
- **Rasterization filter:** 4x supersampling (`8192x6144`) via `resvg`, exact sRGB-to-linear EOTF in `float32`, followed by 4x4 box (area-average) downsampling in linear light to `2048x2048` (avoids ringing and models sensor pixel integration).
- **Unclamped float (`< 0.0`, `> 1.0`):** Visually represented in Row 4 of the SVG and analytically synthesized directly in IEEE 754 `float32` at native `2048x2048` during `export`.
- **Mosaic-only noise:** Row 4 Col 1 (`noisy_flat_fields`) stores clean flat patches in `ground_truth_rgb.tif` and injects deterministic fixed-seed sensor noise (including signed `< 0.0` dark noise) only into `bayer_*.tif`, evaluated separately from clean `[0, 1]` and unclamped-float metrics.
