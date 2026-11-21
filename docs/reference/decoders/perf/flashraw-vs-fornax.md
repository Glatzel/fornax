# FlashRaw versus Fornax raw decode

This is the current decoder performance comparison. It measures only file path
to a storage-native raw mosaic: no active-area crop, `f32` conversion,
preprocessing, hashing, or CSV work is inside the timed region.

The sample set is the stable decoder-path representatives selected from the
[golden performance manifest](../../../../crates/decoder-tracker/golden/perf-manifest.csv),
plus separately measured Nikon path targets and R5 CRX coverage. The current
performance manifest has 33 grouped inputs. Its 20-row decoder-path matrix is
[golden-benchmark-matrix.csv](data/golden-benchmark-matrix.csv). Per-batch
timings, throughput, faults, sample types, and hashes are in
[golden-flashraw-vs-fornax-2026-11-17.csv](data/golden-flashraw-vs-fornax-2026-11-17.csv).
Dated measurement CSVs are immutable snapshots.

## Method

- FlashRaw 0.3.1: `open_path` plus `decode`, strict corruption handling, raw
  output, crop and opcodes disabled, scalar CPU backend.
- Fornax: native `Decoder::decode` plus `raw_mosaic`.
- Target rows use interleaved base/new and FlashRaw processes with warmups and
  repeated rounds. Displayed spread is the range of independent
  release-profile process medians; paired ranges and controls are in review
  evidence rather than this current-results table.
- One and sixteen threads are aligned through `RAYON_NUM_THREADS` and explicit
  FlashRaw worker counts.
- Measurements use warm page cache, `lto=thin`, and `codegen-units=1` on the AMD
  Ryzen 7 5800H WSL2 host: 8 cores/16 threads, 16 visible CPUs, 12 GiB RAM,
  Rust 1.100.0.

## Correctness

All current golden raw mosaics match the accepted Fornax baseline for both
decoders. This includes 16-bit and floating-point DNG, Sony uncompressed and
lossless JPEG, Canon CRX and DNG, Nikon Huffman and packed words, and the Sigma
fp 8/10/12-bit samples. Truncated or invalid streams return errors rather than
being padded and decoded. Nikon speculative chunks fall back to the sequential
kernel unless an emitted symbol boundary exactly matches the preceding
accepted boundary; synthetic truncation, forced-resync-failure, bit-flip, odd
width, and partial-row tests exercise the same fallback.

Historical mismatch investigation and pre-fix hashes are retained in the
[decoder performance archive](../../../archive/reference/decoders/perf/).

## Results

Values are milliseconds; `F` is Fornax, `FR` is scalar-CPU FlashRaw, and spread
is the Fornax one-thread process spread.

Ratio colors: 🟢 ≤1.1x, 🔵 >1.1–1.5x, 🟡 >1.5–2x, 🔴 >2x.

| Golden sample | Path | F 1T | FR 1T | F/FR | F 16T | FR 16T | F/FR | spread |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Canon PowerShot S3IS | DNG uncompressed 16-bit | 3.16 | 2.71 | <span style="color:#0969da">🔵 1.16x</span> | 2.68 | 1.78 | <span style="color:#0969da">🔵 1.50x</span> | 17.7% |
| Canon EOS 5D Mark III | DNG LJPEG 16-bit | 10.86 | 7.31 | <span style="color:#0969da">🔵 1.49x</span> | 3.22 | 1.86 | <span style="color:#9a6700">🟡 1.73x</span> | 8.9% |
| Canon EOS 5D Mark III | DNG uncompressed 16-bit | 51.30 | 50.53 | <span style="color:#1a7f37">🟢 1.02x</span> | 24.18 | 42.72 | <span style="color:#1a7f37">🟢 0.57x</span> | 4.5% |
| Canon EOS 5D Mark III | DNG Deflate FP16 | 178.20 | 135.99 | <span style="color:#0969da">🔵 1.31x</span> | 42.20 | 45.03 | <span style="color:#1a7f37">🟢 0.94x</span> | 5.2% |
| Canon EOS 5D Mark III | DNG uncompressed 24-bit | 55.12 | 49.42 | <span style="color:#0969da">🔵 1.12x</span> | 24.80 | 44.16 | <span style="color:#1a7f37">🟢 0.56x</span> | 3.3% |
| Canon EOS 5D Mark III | DNG Deflate FP24 | 295.34 | 209.06 | <span style="color:#0969da">🔵 1.41x</span> | 54.14 | 44.24 | <span style="color:#0969da">🔵 1.22x</span> | 7.2% |
| Canon EOS 5D Mark III | DNG uncompressed 32-bit | 32.59 | 43.28 | <span style="color:#1a7f37">🟢 0.75x</span> | 24.53 | 46.99 | <span style="color:#1a7f37">🟢 0.52x</span> | 3.3% |
| Canon EOS 5D Mark III | DNG Deflate FP32 | 337.81 | 265.19 | <span style="color:#0969da">🔵 1.27x</span> | 55.41 | 47.47 | <span style="color:#0969da">🔵 1.17x</span> | 4.7% |
| Canon EOS 60D | DNG LJPEG 14-bit | 6.48 | 2.38 | <span style="color:#cf222e">🔴 2.72x</span> | 6.32 | 2.73 | <span style="color:#cf222e">🔴 2.32x</span> | 1.7% |
| Canon EOS R5 | CR3 lossless CRX | 881.51 | 404.32 | <span style="color:#cf222e">🔴 2.18x</span> | 330.31 | 121.01 | <span style="color:#cf222e">🔴 2.73x</span> | 4.8% |
| Canon EOS R6 | CR3 lossless CRX | 156.21 | 71.73 | <span style="color:#cf222e">🔴 2.18x</span> | 59.66 | 20.25 | <span style="color:#cf222e">🔴 2.95x</span> | 5.4% |
| Canon EOS R7 | CR3 lossless CRX | 690.56 | 291.14 | <span style="color:#cf222e">🔴 2.37x</span> | 304.81 | 91.72 | <span style="color:#cf222e">🔴 3.32x</span> | 5.7% |
| Canon PowerShot G1 X | DNG uncompressed 14-bit | 88.90 | 10.52 | <span style="color:#cf222e">🔴 8.45x</span> | 110.23 | 6.60 | <span style="color:#cf222e">🔴 16.71x</span> | 1.5% |
| Nikon Z5 II | NEF Huffman | 34.13 | 33.06 | <span style="color:#1a7f37">🟢 1.03x</span> | 12.36 | 11.18 | <span style="color:#0969da">🔵 1.11x</span> | 3.1% |
| Nikon D6 | NEF mode-10 packed 14-bit | 37.30 | 15.67 | <span style="color:#cf222e">🔴 2.38x</span> | 5.94 | 7.00 | <span style="color:#1a7f37">🟢 0.85x</span> | 0.4% |
| Nikon D850 | NEF Huffman | 142.33 | 141.25 | <span style="color:#1a7f37">🟢 1.01x</span> | 49.26 | 54.29 | <span style="color:#1a7f37">🟢 0.91x</span> | 6.8% |
| Nikon Z7 | NEF mode-10 packed 14-bit | 81.39 | 31.15 | <span style="color:#cf222e">🔴 2.61x</span> | 11.31 | 13.62 | <span style="color:#1a7f37">🟢 0.83x</span> | 1.6% |
| Nikon Z7 | NEF Huffman | 176.89 | 154.22 | <span style="color:#0969da">🔵 1.15x</span> | 52.47 | 54.65 | <span style="color:#1a7f37">🟢 0.96x</span> | 8.1% |
| Nikon Z9 | NEF Huffman | 172.84 | 153.22 | <span style="color:#0969da">🔵 1.13x</span> | 52.79 | 54.47 | <span style="color:#1a7f37">🟢 0.97x</span> | 7.8% |
| Sigma fp | DNG uncompressed 12-bit | 1.01 | 1.28 | <span style="color:#1a7f37">🟢 0.79x</span> | 0.99 | 0.87 | <span style="color:#0969da">🔵 1.14x</span> | 6.3% |
| Sigma fp | DNG uncompressed 10-bit | 1.02 | 1.17 | <span style="color:#1a7f37">🟢 0.87x</span> | 1.06 | 0.86 | <span style="color:#0969da">🔵 1.24x</span> | 6.9% |
| Sigma fp | DNG uncompressed 8-bit | 2.27 | 0.91 | <span style="color:#cf222e">🔴 2.51x</span> | 2.29 | 0.76 | <span style="color:#cf222e">🔴 2.99x</span> | 5.4% |
| Sony ILCE-1 | ARW uncompressed 14-bit | 30.75 | 30.55 | <span style="color:#1a7f37">🟢 1.01x</span> | 12.93 | 14.81 | <span style="color:#1a7f37">🟢 0.87x</span> | 3.5% |
| Sony ILCE-1 | ARW LJPEG 14-bit | 402.37 | 265.50 | <span style="color:#9a6700">🟡 1.52x</span> | 39.75 | 52.01 | <span style="color:#1a7f37">🟢 0.76x</span> | 3.8% |
| Sony ILCE-7RM5 | ARW uncompressed 14-bit | 34.31 | 34.38 | <span style="color:#1a7f37">🟢 1.00x</span> | 14.95 | 17.95 | <span style="color:#1a7f37">🟢 0.83x</span> | 5.5% |
| Sony ILCE-7RM5 | ARW LJPEG 14-bit | 449.70 | 272.69 | <span style="color:#9a6700">🟡 1.65x</span> | 44.52 | 55.87 | <span style="color:#1a7f37">🟢 0.80x</span> | 3.8% |
| Sony ILCE-9M3 | ARW uncompressed 14-bit | 6.09 | 5.68 | <span style="color:#1a7f37">🟢 1.07x</span> | 4.68 | 4.37 | <span style="color:#1a7f37">🟢 1.07x</span> | 2.9% |
| Sony ILCE-9M3 | ARW LJPEG 14-bit | 73.51 | 55.93 | <span style="color:#0969da">🔵 1.31x</span> | 10.23 | 9.49 | <span style="color:#1a7f37">🟢 1.08x</span> | 3.4% |

The final interleaved Nikon/CRX measurements, paired ranges, controls,
throughput, and faults are in
[golden-nikon-crx-ab-2026-11-18.csv](data/golden-nikon-crx-ab-2026-11-18.csv).

The 5D3 uncompressed rows show Fornax faster than FlashRaw at sixteen threads,
but FlashRaw's 16-thread wall time is slower than its one-thread result on this
host. Read those ratios as single-file observations rather than portable
scaling claims. The small Sigma and S3IS rows also have larger relative spread;
their absolute differences are sub-millisecond to low-millisecond.

## Format findings

- **DNG uncompressed** is not one performance class. Direct 8/10/14-bit rows
  remove the generic scratch pass. The 10-bit Sigma row is near parity, while
  the 8-bit row remains 2.51x/2.99x slower because parsing, allocation, and
  fixed DNG decode work dominate its low-millisecond total. The 14-bit G1 X row
  is 8.45x/16.71x slower; `OpcodeList1` conversion, `roundf`, and temporary
  buffers dominate rather than packed-bit unpacking.
- **DNG lossless JPEG** is 1.49-2.72x slower. The small 60D image is
  operationally noisy, but the larger 16-bit 5D3 tile case is a repeatable
  1.49x/1.73x gap.
- **DNG Deflate FP16/24/32** is 1.27-1.41x slower at one thread. At edge
  completion Fornax is slightly faster for FP16 and 1.17-1.22x slower for
  FP24/FP32.
- **Canon CR3 CRX** is 2.18-2.37x slower at one thread and 2.73-3.32x at edge
  completion across R5/R6/R7. Large-page mosaic and plane buffers improve 16T
  by 12-15%, but each sample has one lossless tile with four stateful plane
  tasks, so most 16T workers remain idle.
- **Nikon Huffman** is 1.01-1.15x slower at one thread and 0.91-1.11x at edge
  completion. Independent bit-stream segments resynchronize on exact symbol
  boundaries; the predictor pre-pass computes two states per row before a
  row-parallel curve lookup.
- **Nikon mode-10 packed** is 2.38-2.61x slower at one thread and 0.83-0.85x at
  edge completion. Fixed 14-byte/eight-sample groups and autovectorized stores
  clear the multi-thread target; the one-thread gap remains allocation and
  memory-bandwidth bound.
- **Sony uncompressed** is 1.00-1.07x at one thread and 0.83-1.07x at 16T.
  Native full-strip copies use 16 MiB parallel chunks for large images and
  8 MiB chunks below 64 MiB; the existing row path remains for serial or
  transformed input. **Sony LJPEG** is 1.31-1.65x at one thread and 0.76-1.08x
  at 16T after switching from tile-row tasks to safe per-tile output views.
- **Sigma 12-bit uncompressed** is effectively at parity. The 10-bit sample is
  near parity and the 8-bit sample is bounded by fixed decode overhead.

## Improvement priorities

Ranked by measured impact and implementation leverage. Each item states the
evidence required before work starts.

1. **Canon CRX edge completion.** R5/R6/R7 are 2.73-3.32x slower at 16T, but
   each file has four plane tasks and uses only about 2.3-2.6 CPUs. Require a
   verifiable intra-plane checkpoint or a parser representation that separates
   run/context decisions from reconstruction before code changes. Timing
   reaches its knee at four threads; no fifth exact leaf exists today.
2. **Sony LJPEG one-thread kernel.** A1, 7RM5, and 9M3 remain 1.31-1.65x slower
   at one thread; all three meet parity at 16T. Require a Huffman/refill kernel
   that improves both thread counts without regressing DNG LJPEG controls.
3. **Nikon mode-10 one-thread residual.** D6 and Z7 remain 2.38x and 2.61x
   slower after fixed-group extraction. Further work must improve the row
   kernel or allocation policy at parallel thread counts too; thread-selected
   allocation is not acceptable.
4. **DNG Deflate kernel.** FP32 inflate and predictor reversal dominate the
   one-thread gap. Require a both-thread-count win; zlib-rs was mixed in
   measurement and is not accepted from a one-thread result.
5. **DNG `OpcodeList1` output conversion.** The G1 X profile spends most
   cycles converting a full `f32` mosaic, applying opcode passes, and rounding
   back to `u16`. Require a correctness-focused design outside the packed-row
   kernel.

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
  there is no fifth exact leaf task in the accepted stream scope.
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
- Thread-selected large-page allocation for Nikon packed and Sony LJPEG outputs was removed:
  packed 16T regressed by up to 2.4x and the policy was not a general optimization.
- Nikon Huffman speculation that retained a second full difference plane regressed Z9 16T to about
  304 ms and raised faults; replaced by in-place gapped regions before acceptance.
- Row-parallel Canon CRX placement was neutral to negative (R6 -12.5%) and was removed.
- Historical neutral and regressed decoder experiments are indexed in the
  [decoder performance archive](../../../archive/reference/decoders/perf/);
  do not repeat one without new profile evidence. New candidates are recorded
  here only after measurement.
