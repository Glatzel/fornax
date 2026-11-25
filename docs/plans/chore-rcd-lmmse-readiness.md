# RCD and LMMSE Production Readiness Plan

## Goal

Bring `Demosaic::Rcd` and `Demosaic::Lmmse` to production readiness for Fornax's
signed, unclamped, scene-linear 2x2 RGB Bayer contract (`RawFloat`: `f32` and
`f64`), expose configurable tile-size options with defaults (`RcdOptions` and
`LmmseOptions`), rewrite `rcd.md` and `lmmse.md` with accurate GitHub-safe
LaTeX, evaluate both in `compare-demosaic`, and complete an independent code and
documentation review.

## Scope

In scope:
- `crates/fornax-raw/src/demosaic.rs` (`RcdOptions`, `LmmseOptions`, `Demosaic::rcd()`, `Demosaic::lmmse()`, rustdocs, tests)
- `crates/fornax-raw/src/demosaic/rcd.rs` (energy floor symmetry fix, diagonal HPF & buffer aliasing fix, signed zero-crossing fallback, linear tile scale normalization, Rayon tiling, `RcdOptions`, comprehensive tests + `insta` snapshot)
- `crates/fornax-raw/src/demosaic/lmmse.rs` (remove gamma transfer & unsigned `1.75 * luma` hack, linear Zhang-Wu color differences, scale-relative variance floor, parity-preserving reflected halo, Rayon tiling, `LmmseOptions`, comprehensive tests + `insta` snapshot)
- `crates/fornax-raw-cli/src/config.rs` and `crates/fornax-raw/benches/demosaic.rs`
- `docs/reference/demosaic/{rcd.md,lmmse.md,README.md}`, `docs/README.md`, `docs/plans/chore-rcd-lmmse-readiness.md`
- `crates/compare-demosaic/{src/lib.rs,README.md,scripts/{evaluate.py,mtf.py,run.sh,test_compare_demosaic.py}}`

Out of scope:
- Modifying `ppg.rs`, `ppg.md`, or `TODO.md`
- Reference-library behavioral fidelity or gamma-domain workspaces
- Committing binary files (`temp/compare-demosaic/` stays untracked)

## Task Checklist

- [x] Create `.worktrees/chore-rcd-lmmse-readiness` from `origin/main` and link external fixtures
- [x] Expose `RcdOptions` and `LmmseOptions` with defaults on `Demosaic`
- [x] Fix RCD bugs (energy floor symmetry, diagonal HPF parity, signed zero-crossing fallback, linear tile normalization, Rayon tiling) and add tests
- [x] Redesign LMMSE in sensor-linear space (remove gamma transfer and unsigned highlight hack, fix reflected halo, scale-relative variance floor, Rayon tiling) and add tests
- [x] Rewrite `rcd.md`, `lmmse.md`, and update `docs/reference/demosaic/README.md` & `docs/README.md`
- [x] Add `rcd` and `lmmse` to `compare-demosaic` and run full evaluation
- [x] Dispatch independent reviewer agent to audit code and docs; address findings
- [x] Run `scripts/agent-validation.sh`, `compare-demosaic-test`, and open PR

## Validation

- `INSTA_UPDATE=no cargo test -p fornax-raw --lib demosaic`
- `pixi run -e compare-demosaic compare-demosaic --method all`
- `pixi run -e compare-demosaic compare-demosaic-test`
- `scripts/agent-validation.sh`
- `git diff --check`

## Decisions and Notes

- **RCD signed zero-crossing policy:** When $L_0$ and $L_{2d}$ share sign ($L_0 L_{2d} > 0$), use the sign-safe ratio $F_d \cdot \frac{2|L_0|}{|L_0| + |L_{2d}| + \varepsilon}$; when they cross or touch zero ($L_0 L_{2d} \le 0$), fall back to the first-order additive color-difference estimate $F_d + \frac{1}{2}(L_0 - L_{2d})$ so signed zero-crossing ramps and edges remain exact, scale-equivariant, and singularity-free.
- **LMMSE linear-light policy:** Remove the darktable gamma workspace and unsigned `1.75 * luma` highlight check; evaluate Zhang-Wu (2005) directional color differences, Gaussian low-pass, LMMSE denoising, and variance fusion directly on linear samples with per-tile scale normalization and reflected halos. Keep `LmmseMode` (`Basic`, `Median` default, `ThreeMedians`, `Refine(NonZeroU8)`) in linear light.
- **Configurable tile options:** Expose `RcdOptions { tile_size }` (default `112`) and `LmmseOptions { mode, tile_size }` (default `LmmseMode::Median`, `136`) with `Demosaic::rcd()` and `Demosaic::lmmse()` const constructors.
