# FlashRaw versus Fornax raw decode

This is the current decoder performance comparison. It measures only file path
to a storage-native raw mosaic: no active-area crop, `f32` conversion,
preprocessing, hashing, or CSV work is inside the timed region.

The sample set is the stable decoder-path representatives selected from the
[golden performance manifest](../../../../crates/decoder-tracker/golden/perf-manifest.csv),
plus separately measured Nikon path targets. The current performance manifest
has 33 grouped inputs. Its 20-row decoder-path matrix is
[golden-benchmark-matrix.csv](data/golden-benchmark-matrix.csv). Per-batch
timings, throughput, faults, sample types, and hashes are in
[golden-flashraw-vs-fornax-2026-11-17.csv](data/golden-flashraw-vs-fornax-2026-11-17.csv).
Dated measurement CSVs are immutable snapshots.

## Method

- FlashRaw 0.3.1: `open_path` plus `decode`, strict corruption handling, raw
  output, crop and opcodes disabled, scalar CPU backend.
- Fornax: native `Decoder::decode` plus `raw_mosaic`.
- Non-Nikon rows use three independent processes, two warmups, and five rounds
  per process. The result is the median of process medians; spread is
  `(max process median - min process median) / median`.
- Nikon rows come from interleaved base/new runs. Their displayed one-thread
  spreads are the range of three independent release-profile process medians.
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
being padded and decoded.

Historical mismatch investigation and pre-fix hashes are retained in the
[decoder performance archive](../../../archive/reference/decoders/perf/).

## Results

Values are milliseconds; `F` is Fornax, `FR` is scalar-CPU FlashRaw, and spread
is the Fornax one-thread process spread.

| Golden sample | Path | F 1T | FR 1T | F/FR | F 16T | FR 16T | F/FR | spread |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Canon PowerShot S3IS | DNG uncompressed 16-bit | 3.16 | 2.71 | 1.16x | 2.68 | 1.78 | 1.50x | 17.7% |
| Canon EOS 5D Mark III | DNG LJPEG 16-bit | 10.86 | 7.31 | 1.49x | 3.22 | 1.86 | 1.73x | 8.9% |
| Canon EOS 5D Mark III | DNG uncompressed 16-bit | 51.30 | 50.53 | 1.02x | 24.18 | 42.72 | 0.57x | 4.5% |
| Canon EOS 5D Mark III | DNG Deflate FP16 | 178.20 | 135.99 | 1.31x | 42.20 | 45.03 | 0.94x | 5.2% |
| Canon EOS 5D Mark III | DNG uncompressed 24-bit | 55.12 | 49.42 | 1.12x | 24.80 | 44.16 | 0.56x | 3.3% |
| Canon EOS 5D Mark III | DNG Deflate FP24 | 295.34 | 209.06 | 1.41x | 54.14 | 44.24 | 1.22x | 7.2% |
| Canon EOS 5D Mark III | DNG uncompressed 32-bit | 32.59 | 43.28 | 0.75x | 24.53 | 46.99 | 0.52x | 3.3% |
| Canon EOS 5D Mark III | DNG Deflate FP32 | 337.81 | 265.19 | 1.27x | 55.41 | 47.47 | 1.17x | 4.7% |
| Canon EOS 60D | DNG LJPEG 14-bit | 6.48 | 2.38 | 2.72x | 6.32 | 2.73 | 2.32x | 1.7% |
| Canon EOS R6 | CR3 lossless CRX | 171.83 | 61.48 | 2.80x | 56.88 | 20.48 | 2.78x | 4.3% |
| Canon PowerShot G1 X | DNG uncompressed 14-bit | 88.90 | 10.52 | 8.45x | 110.23 | 6.60 | 16.71x | 1.5% |
| Nikon Z5 II | NEF Huffman | 33.94 | 33.14 | 1.02x | 34.26 | 11.35 | 3.02x | 1.5% |
| Nikon D6 | NEF mode-10 packed 14-bit | 81.64 | 15.26 | 5.35x | 9.70 | 6.91 | 1.40x | 0.4% |
| Nikon D850 | NEF Huffman | 195.16 | 141.43 | 1.38x | 194.69 | 53.46 | 3.64x | 1.9% |
| Nikon Z7 | NEF mode-10 packed 14-bit | 178.02 | 30.47 | 5.84x | 19.95 | 13.40 | 1.49x | 1.0% |
| Nikon Z7 | NEF Huffman | 228.88 | 153.77 | 1.49x | 228.49 | 53.47 | 4.27x | 0.5% |
| Nikon Z9 | NEF Huffman | 226.13 | 154.11 | 1.47x | 226.46 | 53.65 | 4.22x | 0.7% |
| Sigma fp | DNG uncompressed 12-bit | 1.01 | 1.28 | 0.79x | 0.99 | 0.87 | 1.14x | 6.3% |
| Sigma fp | DNG uncompressed 10-bit | 1.02 | 1.17 | 0.87x | 1.06 | 0.86 | 1.24x | 6.9% |
| Sigma fp | DNG uncompressed 8-bit | 2.27 | 0.91 | 2.51x | 2.29 | 0.76 | 2.99x | 5.4% |
| Sony ILCE-1 | ARW uncompressed 14-bit | 28.53 | 28.55 | 1.00x | 32.78 | 15.76 | 2.08x | 4.9% |
| Sony ILCE-7RM5 | ARW uncompressed 14-bit | 34.16 | 34.34 | 0.99x | 37.18 | 21.26 | 1.75x | 2.7% |
| Sony ILCE-9M3 | ARW uncompressed 14-bit | 6.27 | 5.95 | 1.05x | 5.21 | 4.34 | 1.20x | 4.7% |
| Sony ILCE-9M3 | ARW LJPEG 14-bit | 82.85 | 59.02 | 1.40x | 17.32 | 9.78 | 1.77x | 3.2% |

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
- **Canon CR3 CRX** is 2.80x/2.78x slower. The cached leading-zero Rice reader
  leaves coefficient prediction, line reconstruction, and plane placement as
  the dominant gap.
- **Nikon Huffman** is 1.02-1.49x slower at one thread and 3.02-4.27x slower at
  edge completion. Z5 II is at parity at one thread; the larger D850, Z7, and
  Z9 files retain the one-thread gap.
- **Nikon mode-10 packed** is 5.35-5.84x slower at one thread but only
  1.40-1.49x slower at edge completion. Row parallelism closes most of the
  multi-thread gap; the one-thread per-sample unpack remains dominant.
- **Sony uncompressed** varies from parity at one thread to 1.2-2.2x slower at
  edge completion on the larger samples. **Sony LJPEG** is 1.40x/1.77x slower;
  four-component rows stream directly into the two Bayer rows without a
  complete result image, second tile, or reshape.
- **Sigma 12-bit uncompressed** is effectively at parity. The 10-bit sample is
  near parity and the 8-bit sample is bounded by fixed decode overhead.

## Improvement priorities

Ranked by measured impact and implementation leverage. Each item states the
evidence required before work starts.

1. **Nikon mode-10 one-thread kernel.** D6 and Z7 are 5.35x and 5.84x slower;
   the scalar little-endian loop refills and extracts one sample at a time.
   Require a release profile showing the row kernel dominates and a grouped
   prototype that exceeds control drift at one thread without regressing 16T.
2. **Nikon Huffman one-thread gap.** D850, Z7, and Z9 are 1.38-1.49x slower;
   Z5 II is at parity. Suspected costs are checked stream boundaries and the
   per-sample curve lookup. Require a release profile and separate measurements
   for refill, identity-curve, and non-identity lookup changes.
3. **Canon CRX reconstruction and placement.** R6 is 2.80x/2.78x slower. The
   Rice prefix reader is complete; require a cycle split across Rice decode,
   prediction, line reconstruction, placement/output, and allocation before
   changing the scan.
4. **DNG Deflate kernel.** FP32 inflate and predictor reversal dominate the
   one-thread gap. Require a both-thread-count win; zlib-rs was mixed in
   measurement and is not accepted from a one-thread result.
5. **DNG `OpcodeList1` output conversion.** The G1 X profile spends most
   cycles converting a full `f32` mosaic, applying opcode passes, and rounding
   back to `u16`. Require a correctness-focused design outside the packed-row
   kernel.
6. **Sony edge-completion memory behavior.** The uncompressed one-thread path
   is at parity; the remaining 1.19x edge gap points to memory and parallel
   scheduling rather than entropy decoding. Require cache/fault evidence before
   changing allocation or scheduling.

## Notes for future work

- Every Nikon lossless Huffman file in the performance and test manifests has
  one strip, no restart or synchronization markers, and per-row predictors that
  depend on the previous row of the same parity. Parallelism requires
  speculative chunk starts, resynchronization, and predictor reconstruction, as
  FlashRaw does. The estimated upside is 2-4x at 16T; this is a large,
  separately gated design with strict fallback and bounded false-start work.
- The fused Huffman table is 13-bit with 8,192 entries, 32 KiB per table, and is
  built once per decode. Checked boundary semantics are retained so a stream
  ending exactly after the final sample is valid, while ending before it or
  mid-code is a decode error.
- The shared packed-row helper parallelizes at 4,194,304 samples and assigns
  `ceil(height / (threads * 4))` rows per task. Nikon mode-10 keeps this
  threshold and task sizing.
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
- FlashRaw's 16T Huffman path uses about 235 MiB versus about 141 MiB for
  Fornax.
- Historical ARW inventories, DNG evidence, FlashRaw 0.2.0 source comparison,
  crop/cast-era reports, and rejected experiment tables are under the
  [decoder performance archive](../../../archive/reference/decoders/perf/).

## Rejected experiments

- Historical neutral and regressed decoder experiments are indexed in the
  [decoder performance archive](../../../archive/reference/decoders/perf/);
  do not repeat one without new profile evidence. New candidates are recorded
  here only after measurement.
