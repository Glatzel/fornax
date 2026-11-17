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

The 20-row metadata matrix is
[golden-benchmark-matrix.csv](data/golden-benchmark-matrix.csv). Per-batch
timings, spread, faults, sample types, hashes, and throughput are in
[golden-flashraw-vs-fornax-2026-11-17.csv](data/golden-flashraw-vs-fornax-2026-11-17.csv).
The 2026-11-17 CSV retains the removed D850 uncompressed row from the prior
21-row run; the current manifest no longer selects that input. The 2026-11-16
file remains as the previous measurement.

## Correctness

All 21 golden raw mosaics match the accepted Fornax baseline for both decoders.
This includes 16-bit and floating-point DNG, Sony uncompressed and lossless
JPEG, Canon CRX and DNG, Nikon Huffman and packed words, and the Sigma fp
8/10/12-bit samples.

The former Sony 12-bit word-width defect is fixed in the current tree.
Historical mismatch investigation and exact pre-fix hashes are retained in the
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
| Sony ILCE-1 | ARW uncompressed 14-bit | 28.53 | 28.55 | 1.00x | 32.78 | 15.76 | 2.08x | 4.9% |
| Sony ILCE-7RM5 | ARW uncompressed 14-bit | 34.16 | 34.34 | 0.99x | 37.18 | 21.26 | 1.75x | 2.7% |
| Sony ILCE-9M3 | ARW uncompressed 14-bit | 6.27 | 5.95 | 1.05x | 5.21 | 4.34 | 1.20x | 4.7% |
| Sony ILCE-9M3 | ARW LJPEG 14-bit | 82.85 | 59.02 | 1.40x | 17.32 | 9.78 | 1.77x | 3.2% |

The 5D3 uncompressed rows show Fornax faster than FlashRaw at 16 threads, but
FlashRaw's 16-thread wall time is slower than its own one-thread result on this
host. Read those ratios as single-file observations rather than portable
scaling claims. The small Sigma and S3IS rows also have larger relative spread;
their absolute differences are sub-millisecond to low-millisecond.

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

1. **Nikon mode-10 packed one-thread kernel.** Row parallelism closes most of
   the edge-completion gap, but the scalar little-endian unpack remains 5.35x
   and 5.84x slower than FlashRaw at one thread. A grouped 8-sample/14-byte
   kernel is the next distinct optimization.
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

## Nikon Huffman and mode-10 speedups (2026-11-17)

This addendum uses the branch point and the two Nikon changes with
`lto=thin` and `codegen-units=1`. Each implementation was run as separate
processes in ten alternating base/new groups, with two warmups and five rounds
per process. Fornax and FlashRaw were aligned at one and sixteen threads on
the same Ryzen 7 5800H WSL2 host. Times are milliseconds; GB/s uses compressed
input bytes. Paired gain is `base - new`, and its range is the full min-max
width across the ten pairs. Faults are minor-fault medians for base/new/FR.

### One thread

| Golden sample | Base | New | FR | Base GB/s | New GB/s | Paired gain [range] | Faults B/N/FR | New/FR |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Nikon Z5 II Huffman | 135.99 | 33.94 | 33.14 | 0.102 | 0.408 | 102.11 [4.16] | 181/181/182 | 1.02x |
| Nikon D850 Huffman | 629.24 | 195.16 | 141.43 | 0.088 | 0.283 | 434.28 [39.45] | 23,085/23,085/1,623 | 1.38x |
| Nikon Z7 Huffman | 645.07 | 228.88 | 153.77 | 0.091 | 0.257 | 415.30 [11.67] | 23,103/23,103/1,642 | 1.49x |
| Nikon Z9 Huffman | 636.07 | 226.13 | 154.11 | 0.091 | 0.255 | 409.12 [13.10] | 23,081/23,081/1,620 | 1.47x |
| Nikon D6 mode-10 packed | 83.09 | 81.64 | 15.26 | 0.451 | 0.459 | 1.50 [1.14] | 10,726/10,726/1,529 | 5.35x |
| Nikon Z7 mode-10 packed | 181.91 | 178.02 | 30.47 | 0.490 | 0.501 | 4.05 [7.33] | 23,567/23,567/2,106 | 5.84x |
| Sony ILCE-9M3 LJPEG control | 70.64 | 70.14 | 55.72 | 0.252 | 0.253 | 0.53 [6.95] | 230/230/233 | 1.26x |
| Canon R6 CRX control | 190.86 | 190.59 | 71.56 | 0.051 | 0.052 | 0.10 [6.18] | 8,936/8,936/4,859 | 2.66x |
| Sigma fp L DNG control | 307.77 | 308.03 | 255.59 | 0.301 | 0.300 | -1.23 [16.38] | 1,907/1,907/1,909 | 1.21x |

### Sixteen threads

| Golden sample | Base | New | FR | Base GB/s | New GB/s | Paired gain [range] | Faults B/N/FR | New/FR |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Nikon Z5 II Huffman | 135.90 | 34.26 | 11.35 | 0.102 | 0.404 | 101.58 [6.56] | 181/181/280 | 3.02x |
| Nikon D850 Huffman | 632.90 | 194.69 | 53.46 | 0.087 | 0.284 | 438.24 [13.26] | 23,085/23,085/3,579 | 3.64x |
| Nikon Z7 Huffman | 650.96 | 228.49 | 53.47 | 0.090 | 0.257 | 422.50 [17.20] | 23,103/23,103/3,471 | 4.27x |
| Nikon Z9 Huffman | 638.56 | 226.46 | 53.65 | 0.090 | 0.254 | 412.39 [16.85] | 23,081/23,081/3,418 | 4.22x |
| Nikon D6 mode-10 packed | 83.06 | 9.70 | 6.91 | 0.451 | 3.864 | 73.33 [1.69] | 10,726/10,727/1,529 | 1.40x |
| Nikon Z7 mode-10 packed | 183.84 | 19.95 | 13.40 | 0.485 | 4.466 | 163.76 [7.44] | 23,567/23,568/2,106 | 1.49x |
| Sony ILCE-9M3 LJPEG control | 15.04 | 15.09 | 9.57 | 1.182 | 1.178 | 0.08 [1.40] | 232/231/243 | 1.58x |
| Canon R6 CRX control | 74.34 | 73.96 | 20.20 | 0.132 | 0.133 | 0.38 [1.81] | 8,936/8,936/114 | 3.66x |
| Sigma fp L DNG control | 55.61 | 55.48 | 41.49 | 1.663 | 1.667 | 0.08 [2.35] | 1,910/1,909/1,938 | 1.34x |

The control rows stay within 1.23 ms at one thread and 0.38 ms at sixteen
threads. The corresponding largest paired ranges are 16.38 ms and 2.35 ms.
The mode-10 one-thread gains (1.50 ms and 4.05 ms) are therefore not claimed
as speedups; the 8.56x D6 and 9.22x Z7 edge-completion gains are unambiguous.
The fused Huffman table is 32 KiB per table and is built with one 8,192-entry
fill per decode, not per row.

Peak RSS is in MiB:

| Nikon target | B 1T | N 1T | FR 1T | B 16T | N 16T | FR 16T |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Z5 II Huffman | 38.0 | 38.3 | 38.9 | 38.5 | 38.3 | 63.9 |
| D850 Huffman | 140.2 | 140.5 | 141.2 | 140.2 | 140.6 | 234.1 |
| Z7 Huffman | 141.6 | 141.7 | 142.4 | 141.6 | 141.7 | 235.3 |
| Z9 Huffman | 141.3 | 141.4 | 142.2 | 141.4 | 141.6 | 234.5 |
| D6 mode-10 packed | 80.8 | 81.2 | 82.0 | 81.2 | 80.5 | 82.7 |
| Z7 mode-10 packed | 170.5 | 170.6 | 171.2 | 170.7 | 170.0 | 171.7 |

Fornax RSS changed by at most 0.5 MiB on the target rows. At 16 threads
FlashRaw's speculative Huffman path reached 234-235 MiB versus about
141-142 MiB for Fornax.

### Profile and remaining gap

Release `perf record` on D850 at one thread attributed 36.25% of base process
cycles to `decode_symbol`, 21.50% to `read_bits`, and 33.36% to the inlined
decode body containing the predictor, curve lookup, and output write. Probe
setup and hash work accounted for the remainder. After the change,
`decode_huffman_fused` is 67.74%; the old symbol/read/fallback routines together
are 2.57%, so the kernel now dominates its own decode path. Relative to base,
process instructions fell from 18.19B to 5.08B, branch misses from 1.26% to
0.24%, and cache misses from 11.49% to 2.42%.

Fornax is at FlashRaw parity on the small Z5 II but remains 1.38-1.49x slower
on the larger Huffman images. FlashRaw uses the same 13-bit fused width but
also reads the stream as unaligned 64-bit words without the checked slow-reader
boundary object at every row tail, and its identity-curve path avoids a random
curve lookup. Fornax retains exact checked semantics and the production curve
lookup. Mode-10 remains 5.35-5.84x slower at one thread because its scalar
little-endian kernel still refills and extracts one sample at a time; row
parallelism now reduces that to 1.40-1.49x at sixteen threads.

All Nikon NEF raw hashes in the test and performance ledgers are unchanged.
The shared packed-row helper uses the existing 4,194,304-sample threshold and
assigns `ceil(height / (threads * 4))` rows per task. Small fixtures stay
serial.

### Huffman strip follow-up

Every Nikon lossless Huffman file in the performance and test manifests has
one strip, and Nikon mode 3 has no restart or synchronization markers. The
stream is one continuous bit sequence, and each row seeds its two predictors
from the prior row of the same parity. Strips therefore cannot be decoded
independently in the current container. Edge completion requires speculative
chunk starts plus resynchronization, followed by predictor reconstruction, as
FlashRaw does. That is a large, independently gated design with strict fallback
and bounded false-start work. A realistic upside is roughly 2-4x at sixteen
threads, or movement toward the observed FlashRaw 3.02-4.27x advantage; this
change intentionally leaves entropy decoding sequential.
