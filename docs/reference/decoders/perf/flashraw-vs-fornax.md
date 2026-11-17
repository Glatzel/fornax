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

Two full confirmation runs after the last step kept the main effects: G1 X was
84.9-89.5 ms at one thread, Sony LJPEG 82.7-84.0 ms, and CRX 171.8-173.1 ms.
Unchanged-code rows still drifted between full runs by more than their
within-run batch spread, especially the sub-5 ms S3IS/Sigma rows and the
16-thread Sony/5D3 rows. Treat rows outside a step's changed path as unchanged
rather than as reproducible regressions when the confirmation range overlaps.

### Implementation step results

The ordered commits were benchmarked from the same 2026-11-17 baseline. The
8-bit row was already on a direct-copy path when this work started, so its
specialized bulk copy was neutral rather than a new speedup.

| Step | Sample | Before F 1T / F/FR | After F 1T / F/FR | Before F 16T / F/FR | After F 16T / F/FR |
| --- | --- | ---: | ---: | ---: | ---: |
| 1. Direct packed DNG | Sigma fp 10-bit | 4.04 / 3.50x | 1.01 / 0.83x | 3.51 / 3.99x | 0.88 / 1.02x |
| 1. Direct packed DNG | Sigma fp 8-bit | 2.45 / 2.69x | 2.46 / 2.65x | 2.24 / 2.85x | 2.22 / 2.77x |
| 2. DNG 14-bit kernel | Canon G1 X | 90.04 / 8.38x | 84.90 / 7.78x | 106.03 / 17.53x | 104.05 / 16.35x |
| 3. Sony LJPEG rows | Sony ILCE-9M3 LJPEG | 139.75 / 2.35x | 82.67 / 1.43x | 28.36 / 2.94x | 17.33 / 1.74x |
| 3. Guard | Canon 5D3 LJPEG | 10.87 / 1.35x | 10.47 / 1.31x | 3.38 / 1.73x | 3.24 / 1.74x |
| 3. Guard | Canon 60D LJPEG | 6.61 / 2.76x | 6.53 / 2.67x | 6.47 / 2.34x | 6.51 / 2.41x |
| 4. CRX Rice reader | Canon EOS R6 CRX | 250.73 / 4.05x | 171.83 / 2.80x | 74.93 / 3.62x | 56.88 / 2.78x |

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
- **Canon CR3 CRX** improves from 4.10x/3.82x to 2.80x/2.78x with the cached
  leading-zero Rice reader. Coefficient prediction and plane placement now
  dominate the remaining gap.
- **Nikon Huffman** is 4.21x slower at one thread and 12.00x at edge
  completion; **Nikon D6 packed 14-bit** is 4.94x/11.26x. Both retain poor
  scaling relative to FlashRaw.
- **Sony uncompressed** is at one-thread parity and 1.19x slower at edge
  completion. **Sony LJPEG** improves from 2.39x/2.89x to 1.40x/1.77x by
  streaming four-component rows into the two Bayer rows without a complete
  result image, second tile, or reshape.
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
- Historical ARW inventories, DNG evidence, FlashRaw 0.2.0 source comparison,
  crop/cast-era reports, and rejected experiment tables are under the
  [decoder performance archive](../../../archive/reference/decoders/perf/).
  Do not repeat a listed neutral/regressed experiment without new evidence.

## Interleaved review, 2026-11-17

The final full-run slowdown reported for Canon 5D3 FP16 uncompressed was
rechecked with separately built probe binaries in `base/HEAD` alternation,
ten process pairs per row, two warmups and five rounds per process. Each value
below is one process median. Paired delta is `(HEAD / base - 1) * 100`.

| Row | Base ms | HEAD ms | Paired delta | Delta range | Range width |
| --- | ---: | ---: | ---: | ---: | ---: |
| Canon 5D3 FP16 uncompressed | 49.023 | 51.221 | +5.00% | +0.57..+10.92% | 10.34 pp |
| Canon 5D3 FP24 uncompressed | 52.716 | 52.907 | -0.04% | -2.52..+1.95% | 4.47 pp |
| Canon 5D3 FP32 uncompressed | 33.515 | 33.574 | +0.39% | -3.78..+2.01% | 5.79 pp |
| Sony ILCE-9M3 uncompressed | 6.223 | 5.740 | -8.69% | -15.87..+18.57% | 34.44 pp |
| Nikon D6 packed control | 80.501 | 80.447 | +0.22% | -3.30..+1.49% | 4.79 pp |
| Nikon Z5 II Huffman control | 136.877 | 134.818 | -1.09% | -3.94..+1.68% | 5.62 pp |

FP16 is a real binary-layout regression. FP24/FP32 are unchanged. Sony
uncompressed varies far beyond both controls and has no reliable improvement.

| FP16 comparison | Base ms | Candidate ms | Paired delta | Range width |
| --- | ---: | ---: | ---: | ---: |
| base vs d03ef88 | 48.710 | 48.882 | +0.40% | 5.68 pp |
| base vs 80d616e | 48.775 | 48.768 | -0.01% | 5.28 pp |
| base vs d2e87aa | 48.685 | 50.869 | +4.59% | 2.25 pp |
| base vs c149843 | 49.023 | 51.221 | +5.00% | 10.34 pp |

The regression begins at `d2e87aa`. Its hot FP16 float-conversion function is
the same 1278-byte function before and after the commit, but moves from a
64-byte-aligned address (`0x381b40`) to a 32-byte-aligned address (`0x3838e0`).
Only sparse relative-address bytes differ in the function body. This is a code
placement effect, not a changed decode path. The smallest follow-up should
relocate or isolate the four-component LJPEG code to restore the hot-loop
placement; if that cannot be done without another optimization, drop
`d2e87aa`.

| G1 X comparison | Threads | Base ms | 80d616e ms | Paired delta | Range width |
| --- | ---: | ---: | ---: | ---: | ---: |
| G1 X vs 80d616e | 1T | 89.311 | 81.405 | -9.13% | 2.61 pp |
| G1 X vs 80d616e | 16T | 107.985 | 101.471 | -5.08% | 6.21 pp |
| D6 control vs 80d616e | 1T | 79.414 | 79.763 | +0.22% | 3.79 pp |
| Z5 II control vs 80d616e | 1T | 134.811 | 138.266 | +2.38% | 3.55 pp |
| D6 control vs 80d616e | 16T | 79.879 | 79.858 | -0.00% | 4.14 pp |
| Z5 II control vs 80d616e | 16T | 136.044 | 139.235 | +2.28% | 5.52 pp |

The 14-bit kernel is worth keeping: it improves G1 X at both thread counts by
more than the control drift.

Merge recommendations:

- `d03ef88`: merge. Direct packed rows are not implicated in these regressions.
- `80d616e`: merge. G1 X improves 9.13% at 1T and 5.08% at 16T.
- `d2e87aa`: merge after a layout fix, otherwise drop. It causes a repeatable
  ~5% FP16 uncompressed regression despite the large Sony LJPEG gain.
- `c149843`: merge after the d2 layout issue is resolved. The CRX work adds no
  regression in this review.
