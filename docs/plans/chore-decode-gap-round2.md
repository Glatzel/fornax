> Status: active plan.

# Decode-gap round 2 (multithreaded priority)

## Goal

Close the remaining decode gaps against FlashRaw 0.4.0 (scalar CPU backend, release `lto=thin`, `codegen-units=1`), prioritizing 16-thread wall time while preserving bit-exact golden hashes, strict corruption errors, public APIs, safe Rust (no new `unsafe`, no explicit SIMD, no copied code, no new dependencies), and one shared kernel across all thread counts.

## Scope

In scope:

- DNG `OpcodeList1` conservative exact identity elision, zero-copy `FixBadPixelsConstant`, and any remaining 16T leaf bottleneck on G1 X / Leica SL2.
- Pentax 645Z / DNG LJPEG exact (non-speculative) unstuffing and shared-kernel improvements.
- Canon CRX serial tail (`integrate_plane`) and slowest-plane kernel improvements (time-boxed to 2 kernel changes; no speculative intra-plane start).
- DNG Deflate FP24/FP32 per-tile parallelism, reusable worker inflate state, and fused predictor reversal.
- Shared-kernel improvements for Sony LJPEG (`ILCE-1`, `ILCE-7RM5`) and Nikon mode-10 (`D6`, `Z7`) that do not regress 16T.

Out of scope:

- Single-thread-only code paths, branches, or tuning (`threads == 1`).
- Speculative entropy splitting for single-scan LJPEG or Canon CRX.
- Applying `OpcodeList1` in `f32` normalization (changes output; report-only).
- Changing `flate2` inflate backend without explicit user approval.
- Demosaic, public API, or unrelated crate changes.

## Task checklist

- [x] Phase 0: baseline (1T/8T/16T, faults, RSS, CPUs), OpcodeList1 fairness/diff/floor/census, Pentax 645Z split, Nikon D6 regression check, and 16T `perf` leaf profiles.
- [ ] Phase 1: OpcodeList1 identity elision, zero-copy bad-pixel scan, and 16T residual check.
- [ ] Phase 2: Pentax 645Z / DNG LJPEG exact unstuffing / shared-kernel check.
- [ ] Phase 3: Canon CRX serial tail and slowest-plane kernel (time-boxed).
- [ ] Phase 4: DNG Deflate FP24/FP32 per-tile parallelism and fused float predictor.
- [ ] Phase 5: Sony LJPEG and Nikon mode-10 shared-kernel improvements.
- [ ] Final: documentation update (`docs/reference/decoders/perf/flashraw-vs-fornax.md`) and full validation.

## Files expected to change

- `crates/fornax-raw/benches/flashraw_probe/linux.rs`
- `crates/fornax-raw/src/dng/dng_decoder/opcodes.rs`
- `crates/fornax-raw/src/dng/dng_decoder/transforms.rs`
- `crates/fornax-raw/src/dng/dng_decoder/mod.rs`
- `crates/fornax-codec/src/ljpeg.rs`
- `crates/fornax-codec/src/samples.rs` (only if 14-bit DNG unpack or shared helpers need profile-driven fixes)
- `crates/fornax-codec/src/deflate.rs`
- `crates/fornax-raw/src/decoder/canon/codec.rs`
- `crates/fornax-raw/src/decoder/sony/codec/ljpeg.rs`
- `crates/fornax-raw/src/decoder/nikon/codec.rs`
- `docs/plans/chore-decode-gap-round2.md`
- `docs/reference/decoders/perf/flashraw-vs-fornax.md`

## Phase 0 findings

### 0A. OpcodeList1 fairness, diff check, floor, and census

1. **Per-sample diff check (off timed path)**:
   - `Canon/PowerShot G1 X/CRW_0004.DNG` (4496x3366, 15,133,536 u16): `FixBadPixelsConstant(const=0)`. `0` samples equal `0`; `0` samples differ between opcodes applied and opcodes skipped.
   - `LEICA CAMERA AG/LEICA SL2/L1020010.DNG` (8424x5632, 47,443,968 u16): `FixBadPixelsConstant(const=0)`. `0` samples equal `0`; `0` samples differ.
   - `Nokia/Lumia 1020/RAW_NOKIA_LUMIA_1020.DNG` (7152x5360, 38,334,720 u16): four `MapTable` opcodes (covering Bayer phases `(0,0),(0,1),(1,0),(1,1)`), each length `1024` with `table[i] == i` for all `0..1024`. `0` samples differ (`10,094` samples equal `0`, mapped `0 -> 0`).
   - No other materialized DNG in `baseline.csv` or the test/perf manifests has `OpcodeList1`.
2. **FlashRaw with opcodes enabled (`apply_opcodes: true`)**:
   - All three outputs produce the exact same `raw_sha256` as Fornax.
   - G1 X: Fornax `24.79 / 15.16 / 22.87 ms` (1T/8T/16T) vs FlashRaw `op=0`: `10.33 / 7.36 / 6.15 ms` (`2.40x / 2.06x / 3.72x`) vs FlashRaw `op=1`: `11.70 / 8.20 / 7.41 ms` (`2.12x / 1.85x / 3.09x`).
   - Leica SL2: Fornax `72.76 / 37.92 / 50.42 ms` vs FlashRaw `op=0`: `34.48 / 20.61 / 14.93 ms` (`2.11x / 1.84x / 3.38x`) vs FlashRaw `op=1`: `39.81 / 24.31 / 18.84 ms` (`1.83x / 1.56x / 2.68x`).
   - Nokia Lumia 1020: Fornax `265.24 / 52.45 / 63.92 ms` vs FlashRaw `op=0`: `27.10 / 16.40 / 12.17 ms` (`9.79x / 3.20x / 5.25x`) vs FlashRaw `op=1`: `58.09 / 31.71 / 30.98 ms` (`4.57x / 1.65x / 2.06x`).
3. **One-pass floor (`inplace_rw` vs `vec_par_copy` vs `huge_par_copy`)**:
   - G1 X (`15,133,536 u16`): `inplace_rw` = `18.98 / 2.83 / 2.44 ms` (1T/8T/16T); `vec_par_copy` = `4.79 / 5.14 / 5.50 ms`; `huge_par_copy` = `4.50 / 4.65 / 5.60 ms`.
   - Nokia (`38,334,720 u16`): `inplace_rw` = `47.08 / 7.14 / 8.40 ms`; `vec_par_copy` = `66.53 / 20.68 / 20.17 ms`; `huge_par_copy` = `14.59 / 17.79 / 30.68 ms`.
   - Leica SL2 (`47,443,968 u16`): `inplace_rw` = `58.50 / 9.20 / 10.23 ms`; `vec_par_copy` = `77.37 / 24.95 / 25.09 ms`; `huge_par_copy` = `16.88 / 21.95 / 35.44 ms`.
   - Stage split of Fornax `decode_samples` vs `transforms::apply`:
     - G1 X: `7.51+11.32 ms` (1T), `5.75+6.51 ms` (8T), `6.15+24.24 ms` (16T).
     - Leica SL2: `27.66+39.03 ms` (1T), `18.38+19.33 ms` (8T), `29.05+19.81 ms` (16T).
     - Nokia: `20.15+265.06 ms` (1T), `14.93+39.49 ms` (8T), `26.16+40.90 ms` (16T).
   - Note: `fornax_large_alloc::zeroed_samples` (`MADV_HUGEPAGE`) + parallel write across 16 threads regresses severely on Nokia (`30.68ms` vs `vec_par_copy` `20.17ms`) and SL2 (`35.44ms` vs `25.09ms`), and `apply_zero_bad_pixels_constant` also pays `zeroed_samples` + single-thread `copy_from_slice`.
4. **Opcode census across 30 materialized DNGs** (199 additional baseline DNGs remain LFS pointers):
   - `OpcodeList1` (3 files): ID 4 `FixBadPixelsConstant` (G1 X, Leica SL2 — neighborhood), ID 7 `MapTable` (Nokia — pointwise).
   - `OpcodeList2` (4 files): ID 9 `GainMap` (Pixel 8 Pro, Huawei CLT-L09 [all 1.0], Nokia, Sigma fp L [all 1.0] — pointwise).
   - `OpcodeList3` (7 files): ID 1 `WarpRectilinear` (Pixel 8 Pro, Huawei, Leica SL2, Sigma fp 8/10/12, Sigma fp L — geometric, unsupported in `opcodes.rs`), ID 3 `FixVignetteRadial` (iPhone SE — pointwise), ID 9 `GainMap` (Sigma fp 8/10/12/4k [all 1.0] — pointwise).
   - Classification of DNG opcode IDs 1–13:
     - Pointwise: 3 (`FixVignetteRadial`), 7 (`MapTable`), 8 (`MapPolynomial`), 9 (`GainMap`).
     - Row/column-separable: 10 (`DeltaPerRow`), 11 (`DeltaPerColumn`), 12 (`ScalePerRow`), 13 (`ScalePerColumn`).
     - Neighborhood: 4 (`FixBadPixelsConstant`), 5 (`FixBadPixelsList`).
     - Geometric (unsupported by `opcodes.rs`): 1 (`WarpRectilinear`), 2 (`WarpFisheye`), 6 (`TrimBounds`).
   - Current `opcodes.rs` supports IDs 3–5 and 7–13, and rejects 1, 2, 6, and unknown IDs.
5. **Restated Item 1 goal**:
   - **Nokia Lumia 1020**: elide exact identity `MapTable` (`transforms_apply` -> `0 ms`), bringing Fornax to parity with FlashRaw `op=0` (`~12 ms`) and ~`2.5x` faster than FlashRaw `op=1` (`30.98 ms`) at 16T.
   - **G1 X and Leica SL2**: replace `zeroed_samples` + `copy_from_slice` with in-place parallel scan + sparse repair so opcode overhead approaches the `inplace_rw` floor (`~2.4 ms` G1 X, `~10.2 ms` SL2 at 16T) instead of `20–24 ms`, bringing Fornax below `1.5x` vs FlashRaw `op=1` (`7.41 ms` G1 X, `18.84 ms` SL2) — and examine `unpack_row_u16` / allocation if 16T still shows contention (`G1 X` `22.87ms`, `SL2` `50.42ms`).

### 0B. Pentax 645Z: kernel vs scaling

- **1T / 8T / 16T ratios**:
  - `Pentax/645Z/IMGP2836.DNG`: `318.18 / 261.49 (1.22x)` at 1T, `319.05 / 137.77 (2.32x)` at 8T, `322.81 / 126.13 (2.56x)` at 16T (Fornax `1.00` CPU vs FlashRaw `5.08` CPUs, RSS `168.6` vs `399.9 MiB`).
  - Controls: `Pixel 8 Pro`: `1.28x` (1T), `1.38x` (8T), `1.37x` (16T); `Sigma fp L`: `1.21x` (1T), `1.37x` (8T), `1.35x` (16T).
- **Conclusion**: At 1T, 645Z (`1.22x`) matches the other large LJPEG rows (`1.21–1.28x`). The 16T gap (`2.56x`) is a **scaling problem** on its single 65,531,411-byte (`63,953,360` unstuffed) entropy segment (`SOF3=(14, 4192, 6208, 2)`, `pred=1, pt=0`).
- **FlashRaw 16T `perf` split**: `ljpeg_par::Scan::bulk` (speculative entropy) `53.51%` (+ `par_entropy::decode` `1.15%`), `ljpeg::unstuff` (chunk-parallel exact unstuffing) `19.38%` (`14.56%` pass 1 + `4.82%` pass 2), `sum_rows` (parallel predictor rebuild) `8.31%`.
- **Fornax split**: `FastBitReader` unstuffs inline per 8-byte window / per byte (`1,577,977` `0xFF 0x00` sequences = `2.408%` of entropy bytes, hitting `refill_slow` on ~17.7% of 8-byte windows). `seq_unstuff_1t` = `132.96 ms` vs `ljpeg_decode_1t` = `322.30 ms`; 16T `perf` shows `82.55%` in `ljpeg::decode` (Huffman + `refill` + pred), `6.93%` in `refill_slow`, and `2.08%` in slow Huffman.
- **Gate check**: `docs/reference/decoders/design/restart-free-single-scan-parallelism-design.md` forbids splitting the single entropy segment speculatively without approval. Exact chunk-parallel `0xFF 0x00` unstuffing + plain 64-bit word refill is exact by construction and does not split the entropy segment; it adds an unstuffed buffer (~61 MiB on top of 168.6 MiB RSS = ~1.36x RSS, below 1.5x).

### 0C. Nikon mode-10 D6 regression check

- Interleaved 5-batch A/B on `Nikon/D6/D06_0312.NEF`:
  - `ada8360`: 1T `36.72 ms` (`10,726` faults), 16T `5.90 ms` (`10,727` faults).
  - `41625d0`: 1T `36.56 ms` (`10,726` faults), 16T `5.92 ms` (`10,728` faults).
  - `main` (`73fa583`): 1T `36.64 ms` (`10,726` faults), 16T `5.93 ms` (`10,729` faults).
  - FlashRaw `0.4.0`: 1T `14.85 ms` (`1,529` faults), 16T `6.97 ms` (`1,529` faults).
- **Conclusion**: No code regression occurred (`36.72 -> 36.64 ms`). The earlier `1.63x` (`55.16 / 33.85 ms`) vs current `2.47x` (`36.64 / 14.85 ms`) is purely a measurement/host-state difference where FlashRaw dropped `2.28x` (`33.85 -> 14.85 ms`, `9,873 -> 1,529` faults) while Fornax dropped `1.50x` (`55.16 -> 36.64 ms`, `19,100 -> 10,726` faults). 16T leaf profile: `decode_packed` `58.06%`, kernel page faults `~26%`.

### 0D. Baseline, profiles, and summary table

- Note on controls: No Gaussian-named DNG exists in the pinned corpus or manifests (`docs/plans/chore-flashraw-next-gaps.md` confirms the Gaussian DNG control remains unresolved). Float controls are the six 5D3 HDR files; DNG controls are 5D3 uncompressed 16-bit, 5D3 LJPEG (`M24-1221_000202.dng`), Deflate FP16, Sigma fp L, Pixel 8 Pro; Nikon/Sony controls are Z7 Huffman and Sony 9M3 LJPEG.
- CRX per-plane 16T timing:
  - R5: `parse=0.002ms`, `alloc=0.026ms`, `par_planes=208.18ms` (`p0=203.58, p1=203.01, p2=203.05, p3=207.78`, imbalance `1.023x`), `integrate=34.54ms` (14.2%).
  - R6: `parse=0.002ms`, `alloc=0.937ms`, `par_planes=37.72ms` (`p0=36.10, p1=36.72, p2=36.37, p3=37.32`, imbalance `1.034x`), `integrate=4.60ms` (10.6%).
  - R7: `parse=0.003ms`, `alloc=0.026ms`, `par_planes=150.10ms` (`p0=145.48, p1=146.66, p2=146.36, p3=149.66`, imbalance `1.029x`), `integrate=26.24ms` (14.9%).
  - Slowest-plane leaf profile: `decode_line` `48.3–50.2%`, `rice` `38.1–39.6%`, rayon/alloc `~4%`, `integrate_plane` `~2.5%`.
- Deflate FP24/FP32 profile:
  - `5920x3950`: FP24 has `150` tiles (`400x400`, only `10` tile rows -> 8.67 CPUs at 16T); FP32 has `187` tiles (`352x368`, only `11` tile rows -> 9.92 CPUs at 16T).
  - FP24 1T/16T leaf split: `miniz_oxide` `56.6% / 54.2%`, `decode_float_row_into` `32.4% / 36.3%`.
  - FP32 1T/16T leaf split: `miniz_oxide` `64.3% / 63.1%`, `decode_float_row_into` `24.5% / 25.5%`.
- Sony LJPEG profile:
  - A1 1T/16T: `decode` `79.6% / 82.4%`, `refill_slow` `4.5% / 5.2%`, sink `call_mut` `4.2% / 7.1%`.
  - 7RM5 1T/16T: `decode` `82.5% / 85.2%`, `refill_slow` `1.0% / 1.0%`, sink `call_mut` `4.4% / 8.2%`.

| Row | Current 1T | Current 8T | Current 16T | Goal | Dominant cost | Planned change |
| --- | ---: | ---: | ---: | --- | --- | --- |
| Canon G1 X (DNG op=0 / op=1) | 2.40x / 2.12x | 2.06x / 1.85x | 3.72x / 3.09x | <1.5x vs FR op=1 (floor ~2.4ms) | `unpack_row_u16` 55%, `apply_zero_bad_pixels_constant` 28% (alloc+copy) | Zero-copy parallel scan + sparse repair; inspect 14-bit DNG unpack |
| Leica SL2 (DNG op=0 / op=1) | 2.11x / 1.83x | 1.84x / 1.56x | 3.38x / 2.68x | <1.5x vs FR op=1 (floor ~10.2ms) | `unpack_row_u16` 59%, `apply_zero_bad_pixels_constant` 28% (alloc+copy) | Zero-copy parallel scan + sparse repair; inspect 14-bit DNG unpack |
| Nokia Lumia 1020 (DNG op=0 / op=1) | 9.79x / 4.57x | 3.20x / 1.65x | 5.25x / 2.06x | Parity vs FR op=0 (<1.2x) | `apply_band` 36%, `roundf` 26%, `apply_opcode_bands` 22% | Exact identity `MapTable` elision |
| Pentax 645Z (DNG LJPEG) | 1.22x | 2.32x | 2.56x | Improve 16T via exact pre-pass/kernel; no entropy split | Single 65.5 MB entropy scan; `ljpeg::decode` 83%, `refill_slow` 7% | Chunk-parallel exact `FF 00` unstuff + plain 64-bit word refill; 2-comp specialization |
| Canon EOS R5 (CRX) | 2.15x | 2.89x | 2.65x | >=10% 16T gain within 2 kernel changes | `decode_line` 48%, `rice` 40%, serial `integrate_plane` 34.5ms (14%) | Row-parallel contiguous `integrate_plane`; slice sliding window + guarded `CrxBitReader::fill` |
| Canon EOS R6 (CRX) | 2.16x | 2.88x | 2.88x | >=10% 16T gain within 2 kernel changes | `decode_line` 50%, `rice` 39%, serial `integrate_plane` 4.6ms (11%) | Same |
| Canon EOS R7 (CRX) | 2.28x | 3.45x | 3.10x | >=10% 16T gain within 2 kernel changes | `decode_line` 50%, `rice` 38%, serial `integrate_plane` 26.2ms (15%) | Same |
| 5D3 Deflate FP24 | 1.47x | 1.89x | 1.22x | <=1.0x at 16T, FP16/int non-regressing | 10 tile-row tasks (8.67 CPUs), `miniz_oxide` 54%, `decode_float_row_into` 36% | Per-tile disjoint views + worker `Decompress` reuse; fused in-place float predictor |
| 5D3 Deflate FP32 | 1.28x | 1.70x | 1.17x | <=1.0x at 16T, FP16/int non-regressing | 11 tile-row tasks (9.92 CPUs), `miniz_oxide` 63%, `decode_float_row_into` 26% | Same |
| Sony ILCE-1 LJPEG | 1.44x | 1.18x | 0.71x | Keep 16T >=10% faster; 1T <1.3x | Shared LJPEG kernel (`80–82%`), sink `call_mut` (`4–7%`) | Per-worker `FastRowScratch` + direct fused 4-comp row placement + shared LJPEG refill |
| Sony ILCE-7RM5 LJPEG | 1.53x | 1.18x | 0.74x | Keep 16T >=10% faster; 1T <1.3x | Shared LJPEG kernel (`82–85%`), sink `call_mut` (`4–8%`) | Same |
| Nikon D6 mode-10 | 2.41x | 0.75x | 0.86x | Keep 16T >=10% faster; improve 1T | `decode_packed` 41–58%, 10,726 page faults (~15–26%) | Inspect `unpack_row_chunked_14_le` chunk iteration without regressing 16T |
| Nikon Z7 mode-10 | 2.62x | 0.74x | 0.83x | Keep 16T >=10% faster; improve 1T | `decode_packed` 38–58%, 23,569 page faults (~12–26%) | Same |

Controls baseline (`1T / 8T / 16T` Fornax / FlashRaw):
- 5D3 uncompressed 16-bit: `0.97x / 0.88x / 0.93x`
- 5D3 Deflate FP16: `1.40x / 1.26x / 1.00x`
- 5D3 uncompressed FP16: `0.97x / 1.04x / 0.58x`
- 5D3 uncompressed FP24: `1.12x / 1.02x / 0.59x`
- 5D3 uncompressed FP32: `0.75x / 0.97x / 0.57x`
- 5D3 DNG LJPEG: `1.44x / 1.84x / 1.70x`
- Pixel 8 Pro DNG LJPEG: `1.28x / 1.38x / 1.37x`
- Sigma fp L DNG LJPEG: `1.21x / 1.37x / 1.35x`
- Nikon Z7 Huffman: `1.13x / 1.03x / 1.03x`
- Sony 9M3 LJPEG: `1.20x / 1.12x / 1.00x`

## Validation

- Every code commit: golden hashes (`test-manifest.csv` and `perf-manifest.csv`) unchanged, synthetic equivalence tests vs oracle, `scripts/agent-validation.sh` (plus `cargo bench -p fornax-raw --no-run`), 10-alternation interleaved A/B at 1T/8T/16T with untouched controls, peak RSS <= 1.5x, compile time <= +20%.
- Final commit: `docs/reference/decoders/perf/flashraw-vs-fornax.md`, `python3 scripts/check-doc-links.py`, `scripts/agent-validation.sh --docs-only`.

## Decisions and notes

- Phase 0 confirmed no code regression in Nikon D6 (`36.6–36.7 ms` across `ada8360`, `41625d0`, `main`).
- Phase 0 confirmed `OpcodeList1` produces 0 sample diffs on G1 X, Leica SL2, and Nokia Lumia 1020, and FlashRaw `apply_opcodes: true` produces identical hashes.
- Phase 0 confirmed `fornax_large_alloc::zeroed_samples` (`MADV_HUGEPAGE`) + 16T parallel writes cause throughput regression on large DNG buffers (Nokia, SL2).
