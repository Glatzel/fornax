# FlashRaw versus Fornax raw decode

This is the current decoder performance comparison. It measures only file path
to a storage-native raw mosaic: no active-area crop, `f32` conversion,
preprocessing, hashing, or CSV work is inside the timed region.

The sample set is the stable decoder-path representatives selected from the
[golden performance manifest](../../../../crates/decoder-tracker/golden/perf-manifest.csv),
plus separately measured Nikon path targets and R5 CRX coverage. The current
performance manifest has 34 grouped inputs and 19 decoder paths. Its matrix is
[golden-benchmark-matrix.csv](data/golden-benchmark-matrix.csv). Process-level
medians, ranges, faults, sample types, and hashes are in
[fornax-vs-flashraw-0.4.0-2026-11-21.csv](data/fornax-vs-flashraw-0.4.0-2026-11-21.csv).
Dated measurement CSVs are immutable snapshots.

## Method

- FlashRaw 0.4.0: `open_path` plus `decode`, strict corruption handling, raw
  output, crop and opcodes disabled, scalar CPU backend.
- Fornax: native `Decoder::decode` plus `raw_mosaic`.
- Each decoder/thread configuration runs three independent processes. Each
  process performs two warmups and five measured rounds. The displayed value is
  the median of the three process medians; the committed snapshot records the
  process spread, page faults, sample type, and dimensions.
- One and sixteen threads are aligned through `RAYON_NUM_THREADS` and explicit
  FlashRaw worker counts.
- Measurements use warm page cache, `lto=thin`, and `codegen-units=1` on the AMD
  Ryzen 7 5800H WSL2 host: 8 cores/16 threads, 16 visible CPUs, 12 GiB RAM,
  Rust 1.100.0.
- Corpus revision: raw.pixls.us
  `bdef075de28da656976c7cf54f89c1214dbc65d4`.

FlashRaw 0.4.0 was also paired against 0.3.1 on the same 34 inputs, thread
counts, rounds, and corpus. The median 0.4.0/0.3.1 latency ratio was `1.005x`
at one thread and `0.995x` at sixteen threads. No comparable sample moved by
5% in either direction; the 10th-90th percentile range was `0.987-1.029x` at
one thread and `0.965-1.022x` at sixteen. Raw outputs stayed identical. The
per-input evidence is in
[flashraw-0.3.1-vs-0.4.0-2026-11-21.csv](data/flashraw-0.3.1-vs-0.4.0-2026-11-21.csv).
The 0.4.0 release therefore improves cancellation, not uncancelled decode
throughput.

## Correctness

All Fornax rows match the accepted baseline. All 33 FlashRaw-comparable inputs
also produce the same storage-native hash; the Blackmagic 12-bit uncompressed
DNG differs during FlashRaw preflight and is reported as `n/a` rather than
timed. This coverage includes 16-bit and floating-point DNG, Sony uncompressed,
lossless JPEG, and ARW6, Canon CRX and DNG, Nikon Huffman and packed variants,
and the Sigma fp 8/10/12-bit samples.

The broader test golden set was also checked against FlashRaw 0.4.0. Fornax and
FlashRaw agree for its new ARW6, RAF, and 3FR representatives. FlashRaw exposes
different storage-native values for the selected IIQ and RW2 files because
Fornax applies vendor normalization while the reference run disables crop and
opcodes; those rows remain Fornax regression hashes rather than cross-reference
equality claims. Truncated or invalid streams return errors rather than being
padded and decoded.

Historical mismatch investigation and pre-fix hashes are retained in the
[decoder performance archive](../../../archive/reference/decoders/perf/).

## Results

Values are milliseconds as `F / FR (F divided by FR)` with FlashRaw crop and
opcodes disabled (`op=0`); lower ratios favor Fornax. `n/a` means FlashRaw
rejected the input during hash preflight. For the three `OpcodeList1` DNGs,
the `16T (FR op=1)` column also shows FlashRaw with `apply_opcodes: true`.
The speedup column is Fornax `1T / 16T` wall time; values above 1 mean 16T is
faster.

| Format | Camera | Encoding | 1T | 16T | 16T (FR op=1) | 1T/16T speedup |
| --- | --- | --- | ---: | ---: | ---: | ---: |
| DNG | Adobe DNG Canon 5D Mark III | uncompressed 16-bit | 13.41 / 13.40 (1.00x) | 7.81 / 7.88 (0.99x) | — | 1.72x |
| DNG | Blackmagic Pocket Cinema 6K Pro | uncompressed 12-bit | 58.47 / n/a | 59.02 / n/a | — | 0.99x |
| DNG | Canon 5D Mark III | Deflate FP16 | 169.99 / 132.62 (1.28x) | 40.22 / 43.75 (0.92x) | — | 4.23x |
| DNG | Canon 5D Mark III | Deflate FP24 | 282.20 / 206.97 (1.36x) | 52.84 / 44.73 (1.18x) | — | 5.34x |
| DNG | Canon 5D Mark III | Deflate FP32 | 332.69 / 262.36 (1.27x) | 55.52 / 48.10 (1.15x) | — | 5.99x |
| DNG | Canon 5D Mark III | uncompressed FP16 | 42.36 / 47.73 (0.89x) | 23.92 / 40.85 (0.59x) | — | 1.77x |
| DNG | Canon 5D Mark III | uncompressed FP24 | 42.93 / 45.33 (0.95x) | 23.80 / 47.35 (0.50x) | — | 1.80x |
| DNG | Canon 5D Mark III | uncompressed FP32 | 30.29 / 39.04 (0.78x) | 24.86 / 41.24 (0.60x) | — | 1.22x |
| DNG | Canon PowerShot G1 X | uncompressed 14-bit | 10.59 / 10.07 (1.05x) | 6.58 / 6.26 (1.05x) | 6.58 / 7.23 (0.91x) | 1.61x |
| DNG | Google Pixel 8 Pro | lossless JPEG 16-bit | 211.90 / 207.02 (1.02x) | 39.63 / 35.54 (1.12x) | — | 5.35x |
| DNG | HUAWEI CLT-L09 | uncompressed 16-bit | 20.42 / 21.20 (0.96x) | 26.93 / 28.85 (0.93x) | — | 0.76x |
| DNG | Leica SL2 | uncompressed 14-bit | 37.46 / 34.19 (1.10x) | 26.87 / 14.83 (1.81x) | 26.87 / 18.90 (1.42x) | 1.39x |
| DNG | Leica M9 | uncompressed 8-bit | 24.52 / 11.49 (2.13x) | 24.64 / 6.44 (3.83x) | — | 1.00x |
| DNG | Nokia Lumia 1020 | uncompressed 10-bit | 24.79 / 26.99 (0.92x) | 19.16 / 12.08 (1.59x) | 19.16 / 31.32 (0.61x) | 1.29x |
| DNG | Pentax 645Z | lossless JPEG 14-bit | 256.94 / 262.59 (0.98x) | 243.82 / 130.10 (1.87x) | — | 1.05x |
| DNG | Sigma fp | uncompressed 8-bit | 2.26 / 0.95 (2.38x) | 2.23 / 0.75 (2.97x) | — | 1.01x |
| DNG | Sigma fp | uncompressed 10-bit | 1.01 / 1.16 (0.87x) | 1.02 / 0.81 (1.26x) | — | 0.98x |
| DNG | Sigma fp | uncompressed 12-bit | 0.98 / 1.20 (0.82x) | 0.99 / 0.83 (1.19x) | — | 0.99x |
| DNG | Sigma fp L | lossless JPEG 14-bit | 256.83 / 250.21 (1.03x) | 44.90 / 42.10 (1.07x) | — | 5.72x |
| CR3 | Canon EOS R6 | lossless CRX | 136.44 / 70.82 (1.93x) | 54.00 / 20.13 (2.68x) | — | 2.53x |
| CR3 | Canon EOS R7 | lossless CRX | 597.39 / 286.27 (2.09x) | 253.88 / 90.12 (2.82x) | — | 2.35x |
| NEF | Nikon D3 | uncompressed words 14-bit | 6.01 / 5.60 (1.07x) | 5.81 / 5.08 (1.14x) | — | 1.03x |
| NEF | Nikon D6 | mode-10 packed 14-bit | 37.46 / 15.03 (2.49x) | 5.49 / 6.88 (0.80x) | — | 6.83x |
| NEF | Nikon D850 | Huffman 14-bit | 139.62 / 143.04 (0.98x) | 47.82 / 53.19 (0.90x) | — | 2.92x |
| NEF | Nikon Z7 | Huffman 14-bit | 171.66 / 154.51 (1.11x) | 49.09 / 54.15 (0.91x) | — | 3.50x |
| NEF | Nikon Z7 | mode-10 packed 14-bit | 79.68 / 30.33 (2.63x) | 10.78 / 13.35 (0.81x) | — | 7.39x |
| NEF | Nikon Z9 | Huffman 14-bit | 170.81 / 153.64 (1.11x) | 50.16 / 54.55 (0.92x) | — | 3.41x |
| ARW | Sony ILCE-1 | uncompressed 14-bit | 26.02 / 26.42 (0.98x) | 12.24 / 14.14 (0.87x) | — | 2.13x |
| ARW | Sony ILCE-1 | lossless JPEG 14-bit | 306.75 / 257.98 (1.19x) | 31.64 / 50.31 (0.63x) | — | 9.70x |
| ARW | Sony ILCE-7RM4 | uncompressed 12-bit | 30.47 / 30.38 (1.00x) | 14.35 / 17.18 (0.84x) | — | 2.12x |
| ARW | Sony ILCE-7RM5 | uncompressed 14-bit | 30.84 / 30.80 (1.00x) | 14.17 / 17.38 (0.82x) | — | 2.18x |
| ARW | Sony ILCE-7RM5 | lossless JPEG 14-bit | 334.54 / 263.81 (1.27x) | 35.07 / 56.28 (0.62x) | — | 9.54x |
| ARW | Sony ILCE-9M3 | uncompressed 14-bit | 5.17 / 4.95 (1.04x) | 4.51 / 4.35 (1.04x) | — | 1.14x |
| ARW | Sony ILCE-9M3 | lossless JPEG 14-bit | 51.67 / 54.62 (0.95x) | 8.44 / 9.07 (0.93x) | — | 6.13x |

The 5D3 uncompressed rows show Fornax faster than FlashRaw at sixteen threads,
but FlashRaw's 16-thread wall time is slower than its one-thread result on this
host. Read those ratios as single-file observations rather than portable
scaling claims. The small Sigma and S3IS rows also have larger relative spread;
their absolute differences are sub-millisecond to low-millisecond.

## Format findings

- **DNG uncompressed has two distinct profiles.** Direct DNG rows without a
  full-plane transform are at parity or faster: Adobe 16-bit `1.00x/0.99x`,
  Huawei 16-bit `0.96x/0.93x`, and uncompressed floating-point rows
  `0.78-0.95x` at 1T and `0.50-0.60x` at 16T. For the three `OpcodeList1` rows,
  exact identity elision (Nokia `MapTable`), zero-copy parallel
  `FixBadPixelsConstant(const=0)` (G1 X, Leica SL2), and coarse per-thread
  packed-row band sizing (`ceil(height / threads)`) remove the full `f32` /
  `u16` temporary buffers and 16T THP write contention. Against FlashRaw with
  opcodes enabled (`op=1`), Fornax is `0.91x` on G1 X (`6.58 / 7.23 ms`), `1.42x`
  on Leica SL2 (`26.87 / 18.90 ms`), and `0.61x` on Nokia Lumia 1020
  (`19.16 / 31.32 ms`) at 16T (`1.05x`, `1.81x`, and `1.59x` vs `op=0`).
- **DNG lossless JPEG** is `0.98-1.16x` at 1T across the measured rows. At 16T,
  Google Pixel 8 Pro is `1.12x` (`39.63 ms`) and Sigma fp L is `1.07x`
  (`44.90 ms`), while Pentax 645Z improved from `2.56x` to `1.87x` (`243.82 ms`,
  1T `0.98x`, RSS `231 MiB = 1.37x`) via chunk-parallel SWAR `0xFF 0x00`
  unstuffing and direct 1/2-component predictor-1 decoding. Its single
  65.5 MB entropy segment remains the scaling floor (`1.92` CPUs at 16T).
- **DNG Deflate FP16/24/32** is `1.27-1.36x` at 1T and `0.92x / 1.18x / 1.15x`
  at 16T. In-place float predictor reversal (`decode_float_row_in_place_into`)
  plus branchless `extend_fp16` / `extend_fp24` improved both Deflate and
  uncompressed FP paths without adding peak RSS.
- **Canon CR3 CRX** improved to `1.93-2.09x` at 1T and `2.34-2.82x` at 16T
  (R5 `2.34x`, R6 `2.68x`, R7 `2.82x`). Parallel 4-plane band interleaving
  (`integrate_planes_parallel`), slice-window `decode_nontop_line`, and
  branchless `signed(code)` lift 16T CPU utilization to `3.35-3.95` CPUs.
- **Nikon Huffman** is `0.98-1.11x` at 1T and `0.90-0.92x` at 16T.
  Exact-boundary speculative chunks plus row-parallel prediction keep this
  family ahead of FlashRaw at 16T.
- **Nikon mode-10 packed** is `2.49-2.63x` at 1T and `0.80-0.81x` at 16T
  (`5.49 ms` D6, `10.78 ms` Z7). Coarse packed-row band sizing improved 16T
  without regressing peak RSS; 4 KiB page faults (`10,726` / `23,568`) remain
  the 1T floor because `zeroed_samples` (`MADV_HUGEPAGE`) regresses 8T/16T.
- **Sony uncompressed** is `0.98-1.04x` at 1T and `0.82-1.04x` at 16T.
  **Sony LJPEG** improved to `1.19x` (A1), `1.27x` (7RM5), and `0.95x` (9M3) at
  1T, and `0.63x`, `0.62x`, and `0.93x` at 16T via direct 4-component row-pair
  decoding into disjoint tile row views.
- **Small Sigma samples** are dominated by fixed overhead. The 10/12-bit rows
  are faster than FlashRaw at 1T (`0.82-0.87x`) and `1.19-1.26x` at 16T; the
  8-bit row is `2.38x/2.97x` (`2.26 / 2.23 ms` vs `0.95 / 0.75 ms`).

## Improvement priorities

Ranked by measured impact and implementation leverage. Each item states the
evidence required before work starts.

1. **Canon CRX checkpoint.** R5/R6/R7 now reach `3.35-3.95` CPUs (`2.34-2.82x`
   at 16T) after parallel 4-plane interleaving and `decode_nontop_line` cleanup.
   Further 16T scaling requires a verifiable intra-plane checkpoint (bit
   position, Rice state, predictor rows, and global ordinal). The prototype and
   failed checkpoint gate are in `docs/plans/chore-crx-intra-plane-start.md`.
2. **DNG lossless-JPEG single scan.** Pentax 645Z is down to `1.87x` at 16T
   (`0.98x` at 1T) via exact SWAR unstuffing and 2-component predictor-1
   decoding. Closing the remaining 16T gap requires reopening the
   [written boundary-design gate](../design/restart-free-single-scan-parallelism-design.md)
   with explicit user approval for speculative entropy splitting.
3. **DNG OpcodeList1 residual (Leica SL2).** G1 X (`0.91x`) and Nokia
   (`0.61x`) are now faster than FlashRaw `op=1` at 16T; Leica SL2 is `1.42x` vs
   `op=1` (`26.87 ms` vs `10.23 ms` one-pass `inplace_rw` floor) because 16T
   packed-row + in-place scan memory traffic saturates bandwidth.
4. **DNG Deflate FP24/FP32.** FP16 is faster than FlashRaw at 16T (`0.92x`),
   while FP24 (`1.18x`) and FP32 (`1.15x`) remain constrained by `miniz_oxide`
   (~54–63% of cycles) and coarse tile-row count (`10` and `11` tile rows on
   `5920x3950`). Further gains require a faster inflate backend or safe
   intra-band parallelism without THP folio contention.
5. **Nikon mode-10 one-thread residual.** D6 (`2.49x`, `37.46 ms`) and Z7
   (`2.63x`, `79.68 ms`) remain well ahead of FlashRaw at 16T (`0.80x`/`0.81x`),
   with 1T dominated by `10,726`/`23,568` minor 4 KiB page faults. Require an
   allocator or kernel improvement that does not regress 8T/16T via THP
   contention.
6. **Sony LJPEG one-thread kernel.** A1 (`1.19x`) and 7RM5 (`1.27x`) are at
   `0.63x`/`0.62x` at 16T and 9M3 is `0.95x`/`0.93x` across thread counts.

## Notes for future work

- Every Nikon lossless Huffman file in the performance and test manifests has
  one strip, no restart or synchronization markers, and per-row predictors that
  depend on the previous row of the same parity. The parallel decoder merges
  workers only on common exact symbol boundaries inside a bounded 16 KiB
  overlap. Failure to find a boundary or preserve syntax falls back to the
  sequential oracle. Differences temporarily occupy the final `u16` buffer;
  the predictor pass overwrites them after a serial two-values-per-row prepass.
- The fused Huffman table is 13-bit with 8,192 entries, 32 KiB per table, and is
  built once per decode. Checked boundary semantics are retained so a stream
  ending exactly after the final sample is valid, while ending before it or
  mid-code is a decode error.
- The shared packed-row helper parallelizes at 4,194,304 samples and assigns
  coarse per-thread bands of `ceil(height / threads)` rows per task, avoiding
  2 MiB THP folio write contention across threads. Nikon mode-10 keeps this
  threshold and task sizing.
- Nikon's 14-bit little-endian kernel handles eight samples from a fixed
  14-byte chunk using overlapping safe 64-bit loads, then finishes row tails
  scalar. LLVM emits a vectorized widening store loop; the old one-sample
  kernel remains only as a test oracle.
- R5/R6/R7 each contain one lossless tile and four independently coded planes.
  `integrate_planes_parallel` interleaves all four planes per tile-row band,
  lifting 16T CPU utilization to `3.35-3.95` CPUs; `decode_nontop_line` uses
  slice windows and branchless `signed(code)`. Large-page mosaic and plane
  buffers reduce fault cost. There is still no fifth exact leaf task in the
  accepted stream scope; R5's four data bands are 8,852,236, 9,364,150,
  9,366,342, and 8,161,251 bytes, R6's are 1,763,970, 1,818,346, 1,820,223,
  and 1,669,451 bytes, and R7's are 7,094,789, 7,374,054, 7,369,344, and
  6,801,719 bytes.
- Exact identity `OpcodeList1` / `OpcodeList2` / `OpcodeList3` entries
  (verified per opcode type, e.g., identity `MapTable` in Nokia Lumia 1020)
  are elided before conversion or band allocation. `FixBadPixelsConstant(const=0)`
  scans in-place in parallel (`apply_bad_pixels_constant_u16`) and repairs only
  matching pixels without allocating a full-plane temporary.
- Single-scan LJPEG (Pentax 645Z) uses exact SWAR `trailing_zeros` `0xFF 0x00`
  unstuffing into a `zeroed_samples` buffer (`231 MiB` peak RSS = `1.37x`
  prior `169 MiB`) paired with `UnstuffedBitReader` via `FastBits` and direct
  1- and 2-component predictor-1 inner loops (`decode_one_comp_row`,
  `decode_two_comp_row`).
- Sony's four-component LJPEG decodes directly into disjoint tile row-pair views
  via `decode_interleaved_four_into_rows`, eliminating the intermediate tile
  allocation and unpack copy.
- DNG Deflate float decoding reverses predictors in-place
  (`decode_float_row_in_place_into` + `write_float_planes_into`) and extends
  FP16/FP24 via branchless bit manipulation (`extend_fp16` / `extend_fp24`).
- Sony LJPEG builds safe disjoint per-tile row views over the final mosaic, so
  every tile is an independent task without a second full image. DNG LJPEG
  continues to use the shared tile-row scheduler.
- Native Sony uncompressed full strips use 16 MiB parallel copy chunks for
  images at least 64 MiB and 8 MiB chunks below that threshold. Byte swaps,
  masks, multi-strip inputs, and the serial path continue through the row
  kernel.
- Large-page allocation is selected by allocation size, never by Rayon thread
  count. It is retained for Huffman and Canon CRX buffers. Packed and Sony
  LJPEG outputs use plain zeroed vectors because large pages regress their
  parallel write patterns.
- FlashRaw still wins where it uses unaligned 64-bit stream reads and an
  identity-curve fast path. Fornax keeps checked reads and must verify curve
  identity by value once per decode before skipping lookup.

## Retained decisions and archive

- Linux final-output allocation deliberately retains natural alignment and
  `MADV_HUGEPAGE` behind the owned `Vec` API. Invariants and rejected
  caller-owned alternatives are in
  [Linux large-page output allocation](../design/large-page-allocation.md).
- Single-scan LJPEG parallelism remains stopped at the
  [written boundary-design gate](../design/restart-free-single-scan-parallelism-design.md).
- Strict LJPEG end-of-stream detection is within run noise on Sony LJPEG.
- The bench-profile FP16 code-placement regression does not persist in the
  `lto=thin`, `codegen-units=1` release profile.
- FlashRaw's 16T Huffman path uses about 235 MiB. Current Fornax Huffman peak
  RSS is about 181-190 MiB, within the 1.5x growth gate.
- Historical ARW inventories, DNG evidence, FlashRaw 0.2.0 source comparison,
  crop/cast-era reports, and rejected experiment tables are under the
  [decoder performance archive](../../../archive/reference/decoders/perf/).

## Rejected experiments

- `crates/fornax-raw/src/decoders/nikon/codec.rs` checked 64-bit Huffman refill: D850 and
  Z7 regressed 1.7-2.9% at both thread counts; dropped.
- `crates/fornax-raw/src/decoders/nikon/codec.rs` identity-curve fast path: D850 was
  neutral at 1T and slightly slower at 16T; dropped.
- `crates/fornax-raw/src/decoders/canon/codec/mod.rs` 64-bit CRX refill: R5/R6/R7 regressed
  2.2-6.3% at 1T; dropped.
- `crates/fornax-raw/src/decoders/canon/codec/mod.rs` compact line context: neutral at 1T
  and regressed at 16T; dropped.
- `crates/fornax-codec/src/samples.rs` initial Sony uncompressed copy sweeps: one
  large copy regressed 1T by 38-43%; 64 KiB units regressed 16T by 8-21%; the
  retained page-sized parallel rule is separate.
- `crates/fornax-tiff/src/lib.rs` sequential input mapping advice was neutral to negative on Sony
  uncompressed; dropped.
- `crates/fornax-codec/src/ljpeg.rs` 12-bit Huffman lookahead was neutral to negative and regressed
  the Canon 5D3 DNG LJPEG control; dropped.
- `crates/fornax-raw/src/dng/dng_decoder/transforms.rs` 16K, 64K, and 128K
  OpcodeList1 bands improved 1T or 8T but regressed at least one 16T target
  versus the retained 32K band; dropped.
- `crates/fornax-raw/src/dng/dng_decoder/transforms.rs` zero bad-pixel fast path
  using a plain `Vec::to_vec` output raised 16T faults and time; replaced by
  the shared large-page allocator.
- `crates/fornax-codec/src/deflate.rs` tile-first inflate plus row-parallel
  reconstruction regressed FP16/FP24/FP32 at 16T by 8.3%/7.4%/20.1% and raised
  peak RSS to 1.46x/1.55x; dropped.
- `crates/fornax-raw/src/decoders/nikon/codec.rs` paired 14-bit packed
  groups improved 1T by 3.6-4.2% but regressed 16T and left D6 only 8.8%
  faster than FlashRaw; dropped.
- Thread-selected large-page allocation for Nikon packed and Sony LJPEG outputs was removed:
  packed 16T regressed by up to 2.4x and the policy was not a general optimization.
- Nikon Huffman speculation that retained a second full difference plane regressed Z9 16T to about
  304 ms and raised faults; replaced by in-place gapped regions before acceptance.
- Row-parallel Canon CRX placement was neutral to negative (R6 -12.5%) and was removed.
- `crates/fornax-codec/src/samples.rs` `as_chunks` iterators in `unpack_10_msb` / `unpack_14_msb_kernel`: 10-alt A/B neutral at 16T within control drift (`+0.92%` Nokia, `-1.26%` SL2, `+0.08%` G1 X); dropped.
- `crates/fornax-codec/src/ljpeg.rs` `Vec<Vec<u8>>` 2 MiB chunked unstuffing: 16T improved `17.01%` but 1T regressed `+8.91%` (`319.95->347.72ms`) from 4 KiB page faults and slice extensions; replaced by single-buffer `zeroed_samples` SWAR unstuffing.
- `crates/fornax-raw/src/decoder/canon/codec.rs` 64-bit masked bulk `CrxBitReader::fill()`: 10-alt A/B regressed R5 (`+2.45%` 1T, `+2.18%` 16T) and was neutral on R6/R7 (`-1.24%` / `-0.41%` 16T); dropped.
- `crates/fornax-codec/src/deflate.rs` per-tile `TileWork` (`150/187/96` tasks) + reusable `Decompress`: FP24/FP32 16T improved `-5.94%/-6.15%` but FP16 control regressed `+4.48%` from 2 MiB THP folio contention; dropped.
- `crates/fornax-codec/src/deflate.rs` 2-way `rayon::join` per band + reusable `Decompress`: regressed 16T on FP16 (`+3.72%`), FP24 (`+7.28%`), FP32 (`+8.26%`); dropped.
- `crates/fornax-codec/src/deflate.rs` reusable `Decompress` inside `par_chunks_mut(band_stride)`: 10-alt A/B neutral within control drift (`-1.16%` FP24, `-1.32%` FP32, `-0.57%` FP16); dropped.
- `crates/fornax-raw/src/decoder/nikon/codec.rs` `zeroed_samples` in `decode_packed`: 1T improved `-56.69%`/`-60.75%` on D6/Z7 but 8T/16T regressed `+27.18%/+99.62%` (D6) and `+25.46%/+93.10%` (Z7) from THP folio contention; dropped.
- `crates/fornax-raw/src/decoder/nikon/codec.rs` zipped chunk iterator cleanup in `unpack_row_chunked_14_le`: 10-alt A/B neutral within noise (`D6 +0.01%/-0.21%`, `Z7 -0.28%/+0.80%` at 1T/16T); dropped.
- Historical neutral and regressed decoder experiments are indexed in the
  [decoder performance archive](../../../archive/reference/decoders/perf/);
  do not repeat one without new profile evidence. New candidates are recorded
  here only after measurement.
