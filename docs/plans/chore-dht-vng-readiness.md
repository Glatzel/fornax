# DHT, VNG, and VNG4 Production Readiness Plan

## Goal

Bring `Demosaic::Dht`, `Demosaic::Vng`, and `Demosaic::Vng4` to production
readiness for Fornax's signed, unclamped, scene-linear 2x2 RGB Bayer contract
(`RawFloat`: `f32` and `f64`), expose `DhtOptions { tile_size }`, distinguish
3-color `Demosaic::Vng` from 4-color split-green `Demosaic::Vng4`, rewrite
`dht.md` and `vng.md` with accurate GitHub-safe LaTeX, evaluate all three in
`compare-demosaic`, and complete an independent code and documentation review.

## Scope

In scope:
- `crates/fornax-raw/src/demosaic.rs` (`DhtOptions`, `Demosaic::Dht`, `Demosaic::dht()`, `VngLazy`, rustdocs, tests)
- `crates/fornax-raw/src/demosaic/vng.rs` (3-color `Vng` vs 4-color split-green `Vng4`, canonical `minimum + 0.5 * maximum` threshold, parity-preserving reflected borders, streaming `VngLazy` strip execution, comprehensive tests + `insta` snapshots)
- `crates/fornax-raw/src/demosaic/dht.rs` (clean-room linear-light DHT: symmetric 7-tap HV direction costs + 3-pass Jacobi refinement, sign-safe ratio / additive zero-crossing green reconstruction, diagonal direction costs + 1-pass Jacobi refinement, inverse-gradient additive R/B reconstruction, tile-scale normalization, Rayon tiling, `DhtOptions`, comprehensive tests + `insta` snapshot)
- `crates/fornax-raw-cli/src/config.rs` and `crates/fornax-raw/benches/demosaic.rs`
- `docs/reference/demosaic/{dht.md,vng.md,README.md}`, `docs/README.md`, `docs/plans/chore-dht-vng-readiness.md`
- `crates/compare-demosaic/{src/lib.rs,README.md,scripts/{evaluate.py,mtf.py,run.sh,test_compare_demosaic.py}}`

Out of scope:
- Modifying `ppg.rs`, `rcd.rs`, `lmmse.rs`, `amaze.rs`, or `TODO.md`
- LGPL/CDDL LibRaw `dht_demosaic.cpp` translation or hot-pixel hide/restore pass
- Committing binary files (`temp/compare-demosaic/` stays untracked)

## Task Checklist

- [x] Create `.worktrees/chore-dht-vng-readiness` from `origin/chore/amaze-readiness`, link external fixtures, and init pointer-only `raw.pixls.us`
- [x] Implement production-ready 3-color `Vng` and 4-color split-green `Vng4` with `VngLazy`, reflected borders, and `minimum + 0.5 * maximum` threshold
- [x] Implement clean-room linear-light `Demosaic::Dht(DhtOptions)` with Rayon tiling and comprehensive tests
- [x] Wire `dht`, `vng`, and `vng4` into `demosaic.rs`, CLI, bench, and `compare-demosaic`; run full evaluation
- [x] Rewrite `dht.md`, `vng.md`, and update `docs/reference/demosaic/README.md` & `docs/README.md`
- [x] Dispatch independent reviewer agent to audit code and docs; address findings
- [x] Run `scripts/agent-validation.sh`, `compare-demosaic-test`, and open PR

## Validation

- `INSTA_UPDATE=no cargo test --workspace --all-features -- demosaic config`
- `pixi run -e compare-demosaic compare-demosaic --method all`
- `pixi run -e compare-demosaic compare-demosaic-test`
- `scripts/agent-validation.sh`
- `git diff --check`

## Decisions and Notes

- **VNG vs VNG4:** `Demosaic::Vng` is true 3-color VNG (unified green channel, bit-exact measured R/G/B samples, sharper on balanced sensors); `Demosaic::Vng4` is 4-color split-green VNG4 (separate `G1`/`G2` averaged as `(G1 + G2)/2` on output, maze-resistant under green imbalance). Both share one parameterized `VngLazy` streaming kernel with `minimum + 0.5 * maximum` threshold and parity-preserving reflected borders.
- **Clean-room linear-light DHT:** Original Rust formulation without LGPL/CDDL code, `0.5` mid-gray padding, global clamps, or hot-pixel pass. Uses tile-scale normalization, symmetric 7-tap HV directional costs with 3-pass Jacobi majority-vote refinement, RCD-style sign-safe ratio / additive zero-crossing green reconstruction, symmetric diagonal costs with 1-pass Jacobi refinement, and inverse-gradient additive R/B reconstruction.
- **Configurable tile options:** Expose `DhtOptions { tile_size }` (`DEFAULT_TILE_SIZE = 128`) and `Demosaic::dht()` const constructor.
