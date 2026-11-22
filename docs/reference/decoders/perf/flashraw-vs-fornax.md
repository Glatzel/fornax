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

Values are milliseconds as `F / FR (F divided by FR)`; lower ratios favor
Fornax. `n/a` means FlashRaw rejected the input during hash preflight. The
speedup column is Fornax `1T / 16T` wall time; values above 1 mean 16T is
faster.

| Format | Camera | Encoding | 1T | 16T | 1T/16T speedup |
| --- | --- | --- | ---: | ---: | ---: |
| DNG | Adobe DNG Canon 5D Mark III | uncompressed 16-bit | 14.64 / 14.80 (0.99x) | 7.94 / 8.48 (0.94x) | 1.844x |
| DNG | Blackmagic Pocket Cinema 6K Pro | uncompressed 12-bit | 53.91 / n/a | 55.11 / n/a | 0.978x |
| DNG | Canon 5D Mark III | Deflate FP16 | 187.70 / 146.26 (1.28x) | 43.42 / 44.57 (0.97x) | 4.323x |
| DNG | Canon 5D Mark III | Deflate FP24 | 308.44 / 224.49 (1.37x) | 56.01 / 45.74 (1.22x) | 5.507x |
| DNG | Canon 5D Mark III | Deflate FP32 | 360.30 / 289.77 (1.24x) | 58.20 / 48.01 (1.21x) | 6.191x |
| DNG | Canon 5D Mark III | uncompressed FP16 | 71.06 / 73.33 (0.97x) | 25.78 / 41.54 (0.62x) | 2.756x |
| DNG | Canon 5D Mark III | uncompressed FP24 | 87.34 / 83.90 (1.04x) | 26.73 / 43.24 (0.62x) | 3.267x |
| DNG | Canon 5D Mark III | uncompressed FP32 | 73.70 / 87.14 (0.85x) | 25.91 / 39.37 (0.66x) | 2.844x |
| DNG | Canon PowerShot G1 X | uncompressed 14-bit | 25.12 / 11.25 (2.23x) | 23.57 / 6.54 (3.61x) | 1.066x |
| DNG | Google Pixel 8 Pro | lossless JPEG 16-bit | 271.17 / 203.90 (1.33x) | 49.63 / 37.84 (1.31x) | 5.464x |
| DNG | HUAWEI CLT-L09 | uncompressed 16-bit | 22.25 / 23.57 (0.94x) | 26.64 / 30.84 (0.86x) | 0.835x |
| DNG | Leica SL2 | uncompressed 14-bit | 74.41 / 36.03 (2.07x) | 50.99 / 14.86 (3.43x) | 1.459x |
| DNG | Leica M9 | uncompressed 8-bit | 26.23 / 12.25 (2.14x) | 27.23 / 6.35 (4.29x) | 0.963x |
| DNG | Nokia Lumia 1020 | uncompressed 10-bit | 262.67 / 28.73 (9.14x) | 63.52 / 12.32 (5.16x) | 4.135x |
| DNG | Pentax 645Z | lossless JPEG 14-bit | 327.46 / 267.06 (1.23x) | 323.72 / 131.85 (2.46x) | 1.012x |
| DNG | Sigma fp | uncompressed 8-bit | 3.03 / 1.86 (1.63x) | 3.19 / 1.02 (3.13x) | 0.950x |
| DNG | Sigma fp | uncompressed 10-bit | 2.16 / 2.32 (0.93x) | 2.31 / 1.17 (1.98x) | 0.935x |
| DNG | Sigma fp | uncompressed 12-bit | 2.46 / 2.60 (0.95x) | 2.51 / 1.27 (1.98x) | 0.980x |
| DNG | Sigma fp L | lossless JPEG 14-bit | 324.96 / 256.44 (1.27x) | 56.95 / 41.90 (1.36x) | 5.706x |
| CR3 | Canon EOS R6 | lossless CRX | 155.75 / 77.14 (2.02x) | 58.20 / 21.10 (2.76x) | 2.676x |
| CR3 | Canon EOS R7 | lossless CRX | 692.75 / 295.82 (2.34x) | 295.91 / 90.65 (3.26x) | 2.341x |
| NEF | Nikon D3 | uncompressed words 14-bit | 18.31 / 17.48 (1.05x) | 5.77 / 7.60 (0.76x) | 3.173x |
| NEF | Nikon D6 | mode-10 packed 14-bit | 55.16 / 33.85 (1.63x) | 6.96 / 11.33 (0.61x) | 7.925x |
| NEF | Nikon D850 | Huffman 14-bit | 145.67 / 143.94 (1.01x) | 47.90 / 49.97 (0.96x) | 3.041x |
| NEF | Nikon Z7 | Huffman 14-bit | 172.39 / 153.94 (1.12x) | 55.08 / 51.58 (1.07x) | 3.130x |
| NEF | Nikon Z7 | mode-10 packed 14-bit | 80.67 / 30.37 (2.66x) | 12.82 / 13.75 (0.93x) | 6.293x |
| NEF | Nikon Z9 | Huffman 14-bit | 170.43 / 154.12 (1.11x) | 51.75 / 50.80 (1.02x) | 3.293x |
| ARW | Sony ILCE-1 | uncompressed 14-bit | 27.96 / 25.52 (1.10x) | 12.15 / 14.69 (0.83x) | 2.301x |
| ARW | Sony ILCE-1 | lossless JPEG 14-bit | 386.47 / 273.12 (1.41x) | 37.11 / 50.56 (0.73x) | 10.414x |
| ARW | Sony ILCE-7RM4 | uncompressed 12-bit | 86.61 / 84.93 (1.02x) | 22.40 / 26.30 (0.85x) | 3.867x |
| ARW | Sony ILCE-7RM5 | uncompressed 14-bit | 32.07 / 32.30 (0.99x) | 17.41 / 17.31 (1.01x) | 1.842x |
| ARW | Sony ILCE-7RM5 | lossless JPEG 14-bit | 423.86 / 279.29 (1.52x) | 41.08 / 56.17 (0.73x) | 10.318x |
| ARW | Sony ILCE-9M3 | uncompressed 14-bit | 15.40 / 15.21 (1.01x) | 8.10 / 6.42 (1.26x) | 1.901x |
| ARW | Sony ILCE-9M3 | lossless JPEG 14-bit | 92.22 / 67.29 (1.37x) | 13.09 / 10.13 (1.29x) | 7.045x |

The 5D3 uncompressed rows show Fornax faster than FlashRaw at sixteen threads,
but FlashRaw's 16-thread wall time is slower than its one-thread result on this
host. Read those ratios as single-file observations rather than portable
scaling claims. The small Sigma and S3IS rows also have larger relative spread;
their absolute differences are sub-millisecond to low-millisecond.

## Format findings

- **DNG uncompressed has two distinct profiles.** Direct DNG rows without a
  full-plane transform are at parity or faster: Adobe 16-bit `0.99x/0.95x`,
  Huawei 16-bit `0.94x/0.86x`, and floating-point rows `0.62-1.04x`.
  OpcodeList1 rows now use L2-sized bands with per-worker scratch instead of a
  full `f32` mosaic. G1 X is `2.23x/3.61x`, Nokia is `9.14x/5.16x`, and Leica
  SL2 is `2.07x/3.43x` at 1T/16T. The 16T targets remain above 2x; the
  zero-constant bad-pixel fast path removes the remaining full `f32` temporary
  for the two measured FixBadPixelsConstant files.
- **DNG lossless JPEG** is `1.19-1.36x` at one thread across the larger rows.
  At 16T Google and Sigma fp L are `1.31-1.32x`, while Pentax 645Z is `2.49x`.
  Pentax 645Z has one 6208-row, 72,478,678-byte strip, so there is no strip or
  tile scheduler to review; the serial intra-scan entropy path is the floor.
- **DNG Deflate FP16/24/32** is `1.24-1.37x` slower at one thread. FP16 is
  effectively at parity at 16T; FP24/FP32 remain `1.21-1.22x` slower.
- **Canon CR3 CRX** is `2.17-2.37x` slower at one thread and `2.90-3.20x` at
  16T in the current baseline. Large-page mosaic and plane buffers help, but
  each accepted file has one tile with four stateful plane tasks and no fifth
  exact work unit. The four-task path reaches `2.28-2.61` CPUs at 16T.
- **Nikon Huffman** is `1.01-1.12x` at one thread and `0.96-1.07x` at 16T.
  Exact-boundary speculative chunks plus row-parallel prediction keep this
  family near parity.
- **Nikon mode-10 packed** is `1.63-2.66x` slower at one thread and `0.61-0.93x`
  at 16T. Fixed eight-sample extraction clears parallel targets; the remaining
  one-thread gap is row-kernel and allocation work.
- **Sony uncompressed** is `0.99-1.10x` at one thread and `0.83-1.26x` at 16T.
  **Sony LJPEG** is now `1.37-1.52x` slower at one thread. A1 and 7RM5 are
  `0.73x` at 16T; 9M3 remains `1.29x` slower. The retained placement and
  zero-point-transform specializations reduce A1/7RM5 without regressing the
  DNG LJPEG controls.
- **Small Sigma samples** are dominated by fixed overhead. The 10/12-bit rows
  are near parity at one thread but lose `1.98x` at 16T; the 8-bit row is
  `1.63x/3.13x` slower. These are allocation and setup probes, not evidence for
  another packed-row algorithm.

## Improvement priorities

Ranked by measured impact and implementation leverage. Each item states the
evidence required before work starts.

1. **DNG OpcodeList1 residual.** Band execution removes the full `f32` plane,
   but G1 X, Nokia, and Leica SL2 remain `3.61x`, `5.16x`, and `3.43x` slower
   at 16T. Require a new 16T leaf profile showing a dominant remaining band or
   neighborhood cost; do not stack another band-size or allocator tuning
   without that evidence.
2. **Canon CRX checkpoint.** R5/R6/R7 reach only `2.28-2.61` CPUs with four
   exact plane tasks. A speculative intra-plane start remains ineligible until
   bit position, Rice state, predictor rows, and global ordinal have a
   verifiable checkpoint. The prototype and failed checkpoint gate are in
   `docs/plans/chore-crx-intra-plane-start.md`.
3. **Sony LJPEG one-thread kernel.** A1 and 7RM5 are `1.41x/1.52x` slower at
   one thread. Their 16T results are already faster than FlashRaw, so optimize
   the fixed scalar Huffman/refill path without regressing tile scheduling or
   DNG LJPEG controls.
4. **Nikon mode-10 one-thread residual.** D6 and Z7 remain `2.46x/2.60x` slower
   at one thread. Require a row-kernel or allocation win that also improves 16T
   or leaves it at least 10% faster than FlashRaw.
5. **DNG lossless-JPEG single scan.** Pentax 645Z is `2.49x` slower while the
   other large lossless-JPEG rows are `1.31-1.32x`. Its one strip contains the
   only entropy segment; changing the shared kernel now requires the restart-free
   single-scan boundary gate to be reopened with new evidence.
6. **DNG Deflate FP24/FP32.** The one-thread path is `1.24-1.39x` slower and
   FP24/FP32 remain `1.18-1.24x` slower at 16T. Require a both-thread-count win
   with FP16 and integer controls.

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
  `ceil(height / (threads * 4))` rows per task. Nikon mode-10 keeps this
  threshold and task sizing.
- Nikon's 14-bit little-endian kernel handles eight samples from a fixed
  14-byte chunk using overlapping safe 64-bit loads, then finishes row tails
  scalar. LLVM emits a vectorized widening store loop; the old one-sample
  kernel remains only as a test oracle.
- R5/R6/R7 each contain one lossless tile and four independently coded planes.
  Direct placement is used at one thread; the parallel path retains plane
  buffers and interleaving to avoid unsafe aliasing. Large-page mosaic and
  plane buffers reduce fault cost. Timing reaches its knee at four threads;
  there is no fifth exact leaf task in the accepted stream scope. Each plane
  contains one subband; R5's four data bands are 8,852,236, 9,364,150,
  9,366,342, and 8,161,251 bytes, R6's are 1,763,970, 1,818,346, 1,820,223,
  and 1,669,451 bytes, and R7's are 7,094,789, 7,374,054, 7,369,344, and
  6,801,719 bytes.
- The materialized DNG OpcodeList1 census found FixBadPixelsConstant (ID 4)
  in G1 X and Leica SL2 and four MapTable (ID 7) operations in Nokia. No other
  opcode family appeared in the 30 materialized successful DNG rows; 199
  additional DNG baseline rows remain LFS pointers. Neighborhood repairs are
  the fusion barrier: one barrier uses a two-row halo, while multiple barriers
  retain the full-plane oracle.
- Sony's four-component direct path writes two output rows from each decoded
  component pair with fixed-size pair/store loops. The zero point-transform
  specialization removes the per-component transform branch; both changes
  remain in the shared LJPEG kernel.
- Pentax 645Z `IMGP2836.DNG` has one strip, 6208 rows per strip, and a
  72,478,678-byte payload. Its single scan is both the strip task and the only
  entropy segment; there is no scheduler-only fix.
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

- `crates/fornax-raw-decoder/src/decoders/nikon/codec.rs` checked 64-bit Huffman refill: D850 and
  Z7 regressed 1.7-2.9% at both thread counts; dropped.
- `crates/fornax-raw-decoder/src/decoders/nikon/codec.rs` identity-curve fast path: D850 was
  neutral at 1T and slightly slower at 16T; dropped.
- `crates/fornax-raw-decoder/src/decoders/canon/codec/mod.rs` 64-bit CRX refill: R5/R6/R7 regressed
  2.2-6.3% at 1T; dropped.
- `crates/fornax-raw-decoder/src/decoders/canon/codec/mod.rs` compact line context: neutral at 1T
  and regressed at 16T; dropped.
- `crates/fornax-codec/src/samples.rs` initial Sony uncompressed copy sweeps: one
  large copy regressed 1T by 38-43%; 64 KiB units regressed 16T by 8-21%; the
  retained page-sized parallel rule is separate.
- `crates/fornax-tiff/src/lib.rs` sequential input mapping advice was neutral to negative on Sony
  uncompressed; dropped.
- `crates/fornax-codec/src/ljpeg.rs` 12-bit Huffman lookahead was neutral to negative and regressed
  the Canon 5D3 DNG LJPEG control; dropped.
- `crates/fornax-dng/src/dng_decoder/transforms.rs` 16K, 64K, and 128K
  OpcodeList1 bands improved 1T or 8T but regressed at least one 16T target
  versus the retained 32K band; dropped.
- `crates/fornax-dng/src/dng_decoder/transforms.rs` zero bad-pixel fast path
  using a plain `Vec::to_vec` output raised 16T faults and time; replaced by
  the shared large-page allocator.
- `crates/fornax-codec/src/deflate.rs` tile-first inflate plus row-parallel
  reconstruction regressed FP16/FP24/FP32 at 16T by 8.3%/7.4%/20.1% and raised
  peak RSS to 1.46x/1.55x; dropped.
- `crates/fornax-raw-decoder/src/decoders/nikon/codec.rs` paired 14-bit packed
  groups improved 1T by 3.6-4.2% but regressed 16T and left D6 only 8.8%
  faster than FlashRaw; dropped.
- Thread-selected large-page allocation for Nikon packed and Sony LJPEG outputs was removed:
  packed 16T regressed by up to 2.4x and the policy was not a general optimization.
- Nikon Huffman speculation that retained a second full difference plane regressed Z9 16T to about
  304 ms and raised faults; replaced by in-place gapped regions before acceptance.
- Row-parallel Canon CRX placement was neutral to negative (R6 -12.5%) and was removed.
- Historical neutral and regressed decoder experiments are indexed in the
  [decoder performance archive](../../../archive/reference/decoders/perf/);
  do not repeat one without new profile evidence. New candidates are recorded
  here only after measurement.
