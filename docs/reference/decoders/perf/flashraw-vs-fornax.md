# FlashRaw versus Fornax raw decode

This is the current decoder performance comparison. It measures only file path
to a storage-native raw mosaic: no active-area crop, `f32` conversion,
preprocessing, hashing, or CSV work is inside the timed region.

The sample set is the manually curated
[golden performance manifest](../../../../crates/decoder-tracker/golden/perf-manifest.csv).
It selects stable representatives for known-lossless decoder paths across DNG,
ARW, CR3, and NEF, with large inputs where practical and explicitly marked
overhead-only probes for small fixed-cost paths. Lossy modes are not present in
the golden set and are not measured.

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
[golden-flashraw-vs-fornax-2026-11-17.csv](data/golden-flashraw-vs-fornax-2026-11-17.csv).
The 2026-11-16 file remains as the previous measurement.

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
| Nikon Z5 II | NEF Huffman | 137.75 | 32.74 | 4.21x | 135.16 | 11.26 | 12.00x | 2.7% |
| Nikon D6 | NEF packed 14-bit | 80.01 | 16.20 | 4.94x | 79.02 | 7.02 | 11.26x | 3.4% |
| Sigma fp | DNG uncompressed 12-bit | 1.01 | 1.28 | 0.79x | 0.99 | 0.87 | 1.14x | 6.3% |
| Sigma fp | DNG uncompressed 10-bit | 1.02 | 1.17 | 0.87x | 1.06 | 0.86 | 1.24x | 6.9% |
| Sigma fp | DNG uncompressed 8-bit | 2.27 | 0.91 | 2.51x | 2.29 | 0.76 | 2.99x | 5.4% |
| Sony ILCE-9M3 | ARW uncompressed 14-bit | 6.27 | 6.32 | 0.99x | 4.86 | 4.10 | 1.19x | 16.7% |
| Sony ILCE-9M3 | ARW LJPEG 14-bit | 82.85 | 59.02 | 1.40x | 17.32 | 9.78 | 1.77x | 3.2% |

The 5D3 uncompressed rows show Fornax faster than FlashRaw at 16 threads, but
FlashRaw's 16-thread wall time is slower than its own one-thread result on this
host. Read those ratios as single-file observations rather than portable
scaling claims. The small Sigma and S3IS rows also have larger relative spread;
their absolute differences are sub-millisecond to low-millisecond.

### Uncompressed ARW and NEF

These paths use the mapped TIFF source, direct 16-bit-word row output, and a
4-million-sample row-parallel threshold. Smaller mosaics remain serial.

| Row | F 1T | FR 1T | F/FR | F 16T | FR 16T | F/FR | minor faults 1T |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Sony ILCE-1 | 26.48 ms | 26.73 ms | 0.99x | 31.29 ms | 13.98 ms | 2.24x | 2.5k |
| Sony ILCE-7RM5 | 31.20 ms | 31.20 ms | 1.00x | 35.93 ms | 17.44 ms | 2.06x | 2.6k |
| Sony ILCE-9M3 | 5.55 ms | 5.07 ms | 1.09x | 5.17 ms | 4.30 ms | 1.20x | 325 |
| Nikon D850 | 23.72 ms | 23.51 ms | 1.01x | 11.62 ms | 13.02 ms | 0.89x | 2.3k |

Sixteen-thread Sony remains limited by mapped-page residency, memory bandwidth,
and parallel scheduling rather than extra scalar arithmetic.

## Format findings

- **DNG uncompressed** is not one performance class. Direct 8/10/14-bit rows
  remove the generic scratch pass. The 10-bit Sigma row is now near parity, but
  the 8-bit row remains 2.51x/2.99x slower because parsing, allocation, and
  fixed DNG decode work dominate its low-millisecond total. The 14-bit G1 X row
  is 8.45x/16.71x slower; profiling after the specialized kernel attributes
  most remaining time to `OpcodeList1` conversion, `roundf`, and its temporary
  buffers rather than packed-bit unpacking.
- **DNG lossless JPEG** is 1.49-2.72x slower. The tiny 60D image is
  operationally noisy, but the larger 16-bit 5D3 tile case is a repeatable
  1.49x/1.73x gap.
- **DNG Deflate FP16/24/32** is 1.27-1.41x slower at one thread. At edge
  completion Fornax is slightly faster for FP16 and 1.17-1.22x slower for
  FP24/FP32.
- **Canon CR3 CRX** is 2.80x/2.78x slower. The cached leading-zero Rice reader
  leaves coefficient prediction and plane placement as the dominant gap.
- **Nikon Huffman** is 4.21x slower at one thread and 12.00x at edge
  completion; **Nikon D6 packed 14-bit** is 4.94x/11.26x. Both retain poor
  scaling relative to FlashRaw.
- **Sony uncompressed** varies from parity at one thread to 1.2-2.2x slower at
  edge completion on the larger samples. **Sony LJPEG** is 1.40x/1.77x slower;
  four-component rows stream directly into the two Bayer rows without a
  complete result image, second tile, or reshape.
- **Sigma 12-bit uncompressed** is effectively at parity. The 10-bit sample is
  now near parity and the 8-bit sample is bounded by fixed decode overhead.

## Improvement priorities

Ranked by measured impact and implementation leverage:

1. **Nikon fast Huffman and row parallelism.** Use a wider fused
   Huffman/difference table and a 64-bit row kernel for Z5 II Huffman. Apply row
   parallelism to D6 mode-10 packed rows. Current gaps are 4-5x at one thread
   and about 12-13x at edge completion.
2. **Canon CRX residual codec work.** The Rice prefix reader is complete; the
   remaining 2.80x/2.78x gap is coefficient prediction, line reconstruction,
   and plane placement. Require new profile evidence before parallelizing the
   single CRX scan.
3. **DNG Deflate kernel.** FP32 inflate and predictor reversal dominate the
   one-thread gap; zlib-rs was mixed historically and is not accepted without a
   both-thread-count win.
4. **DNG `OpcodeList1` output conversion.** The post-kernel G1 X profile spends
   most cycles converting the full mosaic to `f32`, applying opcode passes, and
   rounding back to `u16`. This is outside the packed-row and 14-bit kernels and
   needs its own correctness-focused design.
5. **Sony edge-completion memory behavior.** The uncompressed one-thread path is
   at parity, so the remaining 1.19x edge gap is memory/parallel scheduling, not
   entropy decoding.

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
- Historical ARW inventories, DNG evidence, FlashRaw 0.2.0 source comparison,
  crop/cast-era reports, and rejected experiment tables are under the
  [decoder performance archive](../../../archive/reference/decoders/perf/).
  Do not repeat a listed neutral/regressed experiment without new evidence.
