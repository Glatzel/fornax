# FlashRaw versus Fornax raw decode

This is the current decoder performance comparison. It measures only file path
to a storage-native raw mosaic: no active-area crop, `f32` conversion,
preprocessing, hashing, or CSV work is inside the timed region.

The sample set is the generated
[golden selected manifest](../../../../crates/decoder-tracker/golden/selected-manifest.csv),
not a hand-picked benchmark list. It contains one lowest-pixel representative
per known-lossless decoder path across DNG, ARW, CR3, and NEF. Lossy modes are
not present in the golden set and are not measured.

## Method

- FlashRaw 0.3.1: `open_path` plus `decode`, strict corruption handling, raw
  output, crop and opcodes disabled, scalar CPU backend.
- Fornax: native `Decoder::decode` plus `raw_mosaic`.
- Three independent processes, two warmups, five rounds per process, median of
  round medians followed by median of batch medians. Spread is
  `(max batch median - min batch median) / median`.
- One and sixteen threads, aligned through `RAYON_NUM_THREADS` and explicit
  FlashRaw worker counts.
- Warm page cache on the AMD Ryzen 7 5800H WSL2 host: 8 cores/16 threads,
  16 visible CPUs, 12 GiB RAM, Rust 1.100.0, optimized bench profile.

The 18-row metadata matrix is
[golden-benchmark-matrix.csv](data/golden-benchmark-matrix.csv). Per-batch
timings, spread, faults, sample types, hashes, and throughput are in
[golden-flashraw-vs-fornax-2026-11-16.csv](data/golden-flashraw-vs-fornax-2026-11-16.csv).

## Correctness

All 18 golden raw mosaics match the accepted Fornax baseline for both decoders.
This includes 16-bit and floating-point DNG, Sony uncompressed and lossless
JPEG, Canon CRX and DNG, Nikon Huffman and packed words, and the Sigma fp
8/10/12-bit samples.

The former Sony 12-bit word-width and Pentax DNG IFD-selection defects are
fixed in the current tree. Historical mismatch investigation and the exact
pre-fix hashes are retained in the
[decoder performance archive](../../../archive/reference/decoders/perf/).

## Results

Values are milliseconds; `F` is Fornax, `FR` is scalar-CPU FlashRaw, and spread
is the Fornax one-thread batch spread.

| Golden sample | Path | F 1T | FR 1T | F/FR | F 16T | FR 16T | F/FR | spread |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Canon PowerShot S3IS | DNG uncompressed 16-bit | 3.28 | 3.06 | 1.07x | 2.92 | 1.60 | 1.82x | 4.0% |
| Canon EOS 5D Mark III | DNG LJPEG 16-bit | 10.61 | 7.60 | 1.40x | 3.34 | 1.81 | 1.84x | 1.3% |
| Canon EOS 5D Mark III | DNG uncompressed 16-bit | 48.35 | 48.55 | 1.00x | 24.20 | 40.43 | 0.60x | 0.8% |
| Canon EOS 5D Mark III | DNG Deflate FP16 | 179.29 | 133.35 | 1.34x | 42.46 | 44.40 | 0.96x | 0.8% |
| Canon EOS 5D Mark III | DNG uncompressed 24-bit | 52.00 | 46.03 | 1.13x | 25.10 | 41.90 | 0.60x | 1.0% |
| Canon EOS 5D Mark III | DNG Deflate FP24 | 298.16 | 205.37 | 1.45x | 53.58 | 45.84 | 1.17x | 1.4% |
| Canon EOS 5D Mark III | DNG uncompressed 32-bit | 30.94 | 40.30 | 0.77x | 25.76 | 42.61 | 0.60x | 2.0% |
| Canon EOS 5D Mark III | DNG Deflate FP32 | 341.95 | 260.69 | 1.31x | 55.81 | 50.26 | 1.11x | 0.6% |
| Canon EOS 60D | DNG LJPEG 14-bit | 6.51 | 2.33 | 2.79x | 6.78 | 2.53 | 2.68x | 2.6% |
| Canon EOS R6 | CR3 lossless CRX | 254.96 | 62.11 | 4.10x | 76.32 | 19.98 | 3.82x | 0.4% |
| Canon PowerShot G1 X | DNG uncompressed 14-bit | 84.72 | 11.00 | 7.70x | 102.52 | 5.90 | 17.38x | 1.6% |
| Nikon Z5 II | NEF Huffman | 136.63 | 33.39 | 4.09x | 137.49 | 10.59 | 12.98x | 1.7% |
| Nikon D6 | NEF packed 14-bit | 81.74 | 15.33 | 5.33x | 80.62 | 6.74 | 11.97x | 9.4% |
| Sigma fp | DNG uncompressed 12-bit | 0.96 | 1.16 | 0.83x | 0.98 | 0.78 | 1.26x | 5.6% |
| Sigma fp | DNG uncompressed 10-bit | 3.24 | 1.15 | 2.81x | 2.88 | 0.87 | 3.31x | 12.0% |
| Sigma fp | DNG uncompressed 8-bit | 2.39 | 0.85 | 2.82x | 2.30 | 0.69 | 3.32x | 13.1% |
| Sony ILCE-9M3 | ARW uncompressed 14-bit | 6.50 | 6.44 | 1.01x | 5.25 | 4.14 | 1.27x | 5.9% |
| Sony ILCE-9M3 | ARW LJPEG 14-bit | 140.24 | 58.79 | 2.39x | 29.06 | 10.05 | 2.89x | 1.5% |

The 5D3 uncompressed rows show Fornax faster than FlashRaw at 16 threads, but
FlashRaw's 16-thread wall time is slower than its own one-thread result on this
host. Read those ratios as single-file observations rather than portable
scaling claims. The small Sigma and S3IS rows also have larger relative spread;
their absolute differences are sub-millisecond to low-millisecond.

## Format findings

- **DNG uncompressed** is not one performance class. The large 5D3 U16/FP
  samples are at parity or faster, but the 14-bit PowerShot G1 X is 7.70x
  slower at one thread and 17.38x slower at edge completion. The 8/10-bit Sigma
  rows are about 2.8x slower. These enter generic packed paths rather than the
  optimized two-byte-word path.
- **DNG lossless JPEG** is 1.31-2.79x slower. The tiny 60D image is
  operationally noisy, but the larger 16-bit 5D3 tile case is a repeatable
  1.40x/1.84x gap.
- **DNG Deflate FP16/24/32** is 1.31-1.45x slower at one thread. At edge
  completion Fornax is slightly faster for FP16 and 1.11-1.17x slower for
  FP24/FP32.
- **Canon CR3 CRX** remains 4.10x slower at one thread and 3.82x at edge
  completion.
- **Nikon Huffman** is 4.09x slower at one thread and 12.98x at edge
  completion; **Nikon D6 packed 14-bit** is 5.33x/11.97x. Both retain poor
  scaling relative to FlashRaw.
- **Sony uncompressed** is at one-thread parity and 1.27x slower at edge
  completion. **Sony LJPEG** is 2.39x/2.89x slower because its four-component
  MCUs still use the generic interleaved path and reshape.
- **Sigma 12-bit uncompressed** is effectively at parity. The 10/8-bit samples
  pay generic packed conversion costs.

## Improvement priorities

Ranked by measured impact and implementation leverage:

1. **Direct DNG packed-row unpacking.** The PowerShot G1 X 14-bit DNG and the
   Sigma 10/8-bit DNGs are the largest ratios relative to their small effort.
   Decode packed samples directly into the final output instead of using the
   generic `u32` scratch-and-convert pass; keep exact bit ordering tests.
2. **Nikon fast Huffman and row parallelism.** Use a wider fused
   Huffman/difference table and a 64-bit row kernel for Z5 II Huffman. Apply row
   parallelism to D6 mode-10 packed rows. Current gaps are 4-5x at one thread
   and about 12-13x at edge completion.
3. **Canon CRX Rice reader.** Replace the unary bit-at-a-time prefix with a
   cached leading-zero reader. CRX is already 4.10x slower and its scaling is
   similar to FlashRaw, so codec efficiency is the primary gap.
4. **DNG 14-bit uncompressed kernel.** Treat this separately from U16 rows. The
   14-bit packed packing order and output conversion should be fused without a
   generic intermediate buffer.
5. **Four-component Sony LJPEG rows.** Keep register-resident predictors and
   write the two 512-wide Bayer rows directly, avoiding the generic result
   buffer, second tile buffer, and reshape.
6. **DNG Deflate kernel.** FP32 inflate and predictor reversal dominate the
   one-thread gap; zlib-rs was mixed historically and is not accepted without a
   both-thread-count win.
7. **Sony edge-completion memory behavior.** The uncompressed one-thread path is
   at parity, so the remaining 1.27x edge gap is memory/parallel scheduling, not
   entropy decoding.

## Retained decisions and archive

- Linux final-output allocation deliberately retains natural alignment and
  `MADV_HUGEPAGE` behind the owned `Vec` API. Invariants and rejected
  caller-owned alternatives are in
  [Linux large-page output allocation](../design/large-page-allocation.md).
- Single-scan LJPEG parallelism remains stopped at the
  [written boundary-design gate](../design/restart-free-single-scan-parallelism-design.md).
- Historical ARW inventories, DNG evidence, FlashRaw 0.2.0 source comparison,
  crop/cast-era reports, and rejected experiment tables are under the
  [decoder performance archive](../../../archive/reference/decoders/perf/).
  Do not repeat a listed neutral/regressed experiment without new evidence.
