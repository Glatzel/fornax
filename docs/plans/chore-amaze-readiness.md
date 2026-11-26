# AMaZE Linear-Light Readiness Plan

- Branch: `chore/amaze-readiness`
- Scope: Make AMaZE (`crates/fornax-raw/src/demosaic/amaze.rs`) production-ready for Fornax's signed, unclamped, scale-equivariant scene-linear 2x2 RGB Bayer contract (`RawFloat`: `f32` and `f64`), expose `AmazeOptions { tile_size }`, update `docs/reference/demosaic/amaze.md` and index docs, and integrate `amaze` into `crates/compare-demosaic`.

## Checklist

- [x] Expose `AmazeOptions { tile_size }` (`DEFAULT_TILE_SIZE = 160`) and `Demosaic::amaze()` in `crates/fornax-raw/src/demosaic.rs`, and update CLI/bench call sites (`crates/fornax-raw-cli/src/config.rs`, `crates/fornax-raw/benches/demosaic.rs`).
- [x] Fix AMaZE implementation defects in `crates/fornax-raw/src/demosaic/amaze.rs`:
  - [x] Parity-preserving `reflect_index` border handling for all image sizes (remove `<= 32` PPG fallback and broken corner/border edge-copy logic).
  - [x] Scale-normalized linear tile processing (`scale = normalization_scale(input)`) with zero-initialized per-tile Rayon scratch buffers.
  - [x] Remove `pair(index)` half-indexing and use direct `tile_size`-strided indexing with `± 2 * ts` / `± 2` neighbor offsets where same-color neighbors are read.
  - [x] Fix `0.25` variance mean factor in horizontal/vertical directional weights.
  - [x] Fix `interpolate_chroma` swapped/sign-flipped offsets (`weight_sw`/`weight_se` and `0.075` diagonal terms) and extend `split_green_differences` across the full `11..tile - 11` range read by `interpolate_chroma`.
  - [x] Guard `write_rgb` denominator against zero and replace signed/HDR-unsafe ratio poles, arbitrary `0.8` highlight threshold, and `[0, signal_scale]` clamps with sign-safe `color_ratio`/`diagonal_ratio` and `clamp_toward_neighbors`.
  - [x] Use out-of-place Jacobi buffer swaps for `select_color_differences`, `interpolate_green_at_red_blue` weights, and `refine_green_from_red_blue` weights.
  - [x] Preserve measured CFA samples bit-exactly in `output`.
- [x] Add comprehensive AMaZE tests (signed, HDR, scale equivariance, sign symmetry, H+V flip, tile-size consistency, `f64`, and 24x24 `insta` snapshot).
- [x] Rewrite `docs/reference/demosaic/amaze.md` with GitHub-safe LaTeX and update `docs/reference/demosaic/README.md` and `docs/README.md`.
- [x] Integrate `amaze` into `crates/compare-demosaic` and verify all tiles and tests pass.
- [x] Run independent read-only reviewer subagent, address findings, run full validation, and open PR.
