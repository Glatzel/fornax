# Fornax vs FlashRaw raw decode

This note compares native Fornax and FlashRaw 0.3.1 from file path to an owned
raw mosaic. It excludes crop, cast, preprocessing, hashing, and CSV work from
the timed region.

## Scope and method

- Samples: the 14 lossless/uncompressed DNG, ARW, CR3, and NEF entries in
  [flashraw-0.3.1-bench-manifest.csv](data/flashraw-0.3.1-bench-manifest.csv).
- FlashRaw: `open_path` plus `decode` with `RawU16`/`RawF32`, strict corruption
  handling, scalar `Backend::Cpu`, crop disabled, and opcodes disabled.
- Fornax: `fornax_raw_decoder::Decoder::decode` plus `raw_mosaic`. Fornax does
  not expose a separate open API; `decode_ms` is therefore its total.
- FlashRaw `open_ms` includes opening/mapping and container parsing. Its
  `decode_ms` starts after that and excludes the following hash.
- Three independent processes, two warmups, five rounds, medians of round
  medians then batch medians. Spread is `(max batch median - min batch median)
  / median`.
- One- and sixteen-thread runs use the AMD Ryzen 7 5800H WSL2 host from the
  preceding FlashRaw note: 8 cores/16 threads, 16 visible CPUs, 12 GiB RAM,
  Rust 1.100.0, optimized bench profile, warm page cache.
- Throughput is raw output bytes at two bytes/sample divided by total time.

The complete per-sample aggregate, batch spread, faults, hashes, and exclusion
reason is [fornax-vs-flashraw-raw-decode-2026-11-16.csv](data/fornax-vs-flashraw-raw-decode-2026-11-16.csv).
Fornax supported all 14 selected samples; there were no unsupported skips.

Against the earlier FlashRaw-only `decode` table, compressed results agree
within roughly 0-5%; uncompressed results are 5-22% faster in this session
(most visibly D850). The old note explicitly warns that wall values are
session-specific, and the old adapter also constructed its `RawMosaic` inside
the timed call. The current three batches are tighter, so these values replace
the old totals for cross-decoder comparison; the historical CSV remains intact.

## Correctness

FlashRaw and Rawler agree bit-for-bit on every mismatch. Fornax is the suspect
decoder in both cases.

| Sample | Fornax | FlashRaw | Rawler | Finding |
| --- | --- | --- | --- | --- |
| Pentax K-1 LJPEG | `8f6d4618…` | `fb04a11b…` | equals FlashRaw | Fornax differs at 36,387,315 / 36,590,400 samples (99.45%). First: index 0, row 0/col 0, 124 vs 129. |
| Sony ILCE-7RM4 uncompressed | `434b92fd…` | `9533fd9c…` | equals FlashRaw | Fornax differs at 61,164,804 / 61,209,600 samples (99.93%). First: index 0, row 0/col 0, 3456 vs 3288. |

The Pentax file has no DNG linearization table, opcode list, or interleave
factor. Every row and all column phases differ, with small signed differences;
this points to Fornax's LJPEG reconstruction, not a DNG transform. The accepted
Fornax baseline hash for this file records the same incorrect output.

The Sony file declares 12 significant bits but stores two-byte uncompressed
words. Fornax promotes only 14/16-bit declared uncompressed ARW to 16-bit words;
this sample enters the 12-bit packed path instead. FlashRaw selects its 16-bit
unpack path. This is the likely root cause.

All other 12 samples match. Leica SL2 is hash-correct but excluded from the
performance table because Fornax has no opcode-disable option and spends time
applying its `OpcodeList1`; FlashRaw was measured with opcodes disabled.

## Per-sample results

Excluded: Pentax and Sony ILCE-7RM4 (wrong Fornax output), and Leica SL2
(non-equivalent opcode work). `F` and `FR` columns are milliseconds; `spread`
is percent. No included difference falls inside the larger batch spread, though
Sigma fp is operationally noise-sized in absolute time.

| Sample | Mode | T | Fornax | spread | FlashRaw | spread | Ratio | F GB/s | FR GB/s |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Sony ILCE-7RM5 | ARW LJPEG | 1 | 923.34 | 0.6% | 278.90 | 2.7% | 3.31x | 0.14 | 0.46 |
| Sony ILCE-7RM5 | ARW LJPEG | 16 | 269.33 | 0.9% | 54.90 | 1.2% | 4.91x | 0.48 | 2.36 |
| Sony ILCE-9M3 | ARW LJPEG | 1 | 179.48 | 1.4% | 57.80 | 1.9% | 3.10x | 0.14 | 0.44 |
| Sony ILCE-9M3 | ARW LJPEG | 16 | 59.48 | 2.9% | 9.85 | 4.6% | 6.04x | 0.42 | 2.56 |
| Sony ILCE-1 | ARW uncompressed | 1 | 222.60 | 1.7% | 28.32 | 0.4% | 7.86x | 0.45 | 3.54 |
| Sony ILCE-1 | ARW uncompressed | 16 | 216.15 | 0.6% | 14.11 | 1.4% | 15.32x | 0.46 | 7.11 |
| Canon EOS R5 II | CR3 CRX | 1 | 1614.27 | 2.7% | 404.72 | 1.2% | 3.99x | 0.06 | 0.24 |
| Canon EOS R5 II | CR3 CRX | 16 | 581.88 | 1.9% | 126.09 | 4.4% | 4.61x | 0.16 | 0.76 |
| Canon EOS R6 | CR3 CRX | 1 | 248.74 | 0.4% | 60.73 | 5.0% | 4.10x | 0.07 | 0.28 |
| Canon EOS R6 | CR3 CRX | 16 | 74.44 | 1.0% | 20.33 | 1.8% | 3.66x | 0.23 | 0.84 |
| Google Pixel 8 Pro | DNG LJPEG | 1 | 263.49 | 0.6% | 204.59 | 3.0% | 1.29x | 0.38 | 0.49 |
| Google Pixel 8 Pro | DNG LJPEG | 16 | 48.85 | 0.4% | 37.41 | 2.0% | 1.31x | 2.05 | 2.68 |
| Sigma fp | DNG uncompressed | 1 | 0.94 | 14.2% | 1.19 | 6.9% | 0.79x | 4.48 | 3.54 |
| Sigma fp | DNG uncompressed | 16 | 0.92 | 2.8% | 0.89 | 3.6% | 1.04x | 4.57 | 4.76 |
| Nikon Z5 II | NEF Huffman | 1 | 165.75 | 1.4% | 33.40 | 3.0% | 4.96x | 0.13 | 0.64 |
| Nikon Z5 II | NEF Huffman | 16 | 165.11 | 2.1% | 10.96 | 2.2% | 15.07x | 0.13 | 1.95 |
| Nikon Z7 | NEF Huffman | 1 | 743.38 | 1.2% | 153.17 | 1.5% | 4.85x | 0.12 | 0.60 |
| Nikon Z7 | NEF Huffman | 16 | 738.97 | 0.3% | 51.12 | 4.8% | 14.46x | 0.12 | 1.79 |
| Nikon D500 | NEF uncompressed | 1 | 113.33 | 2.4% | 13.05 | 0.6% | 8.68x | 0.37 | 3.20 |
| Nikon D500 | NEF uncompressed | 16 | 108.95 | 3.1% | 7.18 | 12.2% | 15.18x | 0.38 | 5.82 |
| Nikon D850 | NEF uncompressed | 1 | 242.98 | 4.3% | 25.53 | 0.5% | 9.52x | 0.38 | 3.58 |
| Nikon D850 | NEF uncompressed | 16 | 238.91 | 1.0% | 13.55 | 3.5% | 17.63x | 0.38 | 6.75 |

## Format summary and scaling

Medians over included samples:

| Format/mode | T | Fornax/FlashRaw | Fornax GB/s | FlashRaw GB/s |
| --- | ---: | ---: | ---: | ---: |
| ARW LJPEG | 1 | 3.21x | 0.14 | 0.45 |
| ARW LJPEG | 16 | 5.47x | 0.45 | 2.46 |
| ARW uncompressed | 1 | 7.86x | 0.45 | 3.54 |
| ARW uncompressed | 16 | 15.32x | 0.46 | 7.11 |
| CR3 CRX | 1 | 4.04x | 0.06 | 0.26 |
| CR3 CRX | 16 | 4.14x | 0.20 | 0.80 |
| DNG LJPEG | 1 | 1.29x | 0.38 | 0.49 |
| DNG LJPEG | 16 | 1.31x | 2.05 | 2.68 |
| DNG uncompressed | 1 | 0.79x | 4.48 | 3.54 |
| DNG uncompressed | 16 | 1.04x | 4.57 | 4.76 |
| NEF Huffman | 1 | 4.91x | 0.13 | 0.62 |
| NEF Huffman | 16 | 14.76x | 0.13 | 1.87 |
| NEF uncompressed | 1 | 9.10x | 0.37 | 3.39 |
| NEF uncompressed | 16 | 16.41x | 0.38 | 6.29 |

One-to-sixteen-thread speedup:

| Format/mode | Fornax | FlashRaw | Interpretation |
| --- | ---: | ---: | --- |
| ARW LJPEG | 3.22x | 5.48x | slower per thread and weaker scaling |
| ARW uncompressed | 1.03x | 2.01x | single tile collapses to one task |
| CR3 CRX | 3.06x | 3.10x | similar scaling; kernel is slower |
| DNG LJPEG | 5.39x | 5.47x | similar scaling; kernel is 29% slower |
| DNG uncompressed | 1.02x | 1.34x | both mostly serial at this size |
| NEF Huffman | 1.00x | 3.02x | Fornax decoder is single-threaded |
| NEF uncompressed | 1.03x | 1.85x | Fornax rows/strips are serial |

The 16-thread gap combines codec efficiency and parallelism. NEF Huffman and
uncompressed remain virtually one-thread in Fornax, so their 14-18x 16-thread
ratios are not explained by per-sample codec speed alone.

## Profiling and source comparison

- NEF Huffman: `perf` attributes 37.9% to `HuffmanTable::decode_symbol`, 22.1%
  to `BitReader::read_bits`, and 33.4% to the Nikon codec. Whole-probe
  instructions are 18.24B vs 4.40B and branches 3.35B vs 0.48B for
  Fornax/FlashRaw. Fornax uses a 10-bit direct table then a bit-at-a-time
  fallback and serial predictor loop. FlashRaw uses a 13-bit fused
  code+difference table, 64-bit windows, four-sample groups, and speculative
  parallel entropy chunks.
- Canon CRX: 38.8% `BitReader::read_bits`, 30.3% decode closure, and 23.9%
  `PlaneDecoder::rice`. Fornax reads a Rice unary prefix one bit at a time.
  FlashRaw's `CrxBits` keeps 56+ bits, extracts a complete Rice word with one
  `leading_zeros`, and keeps line context in compact arrays. Whole-probe
  instructions are 34.03B vs 11.92B and branches 6.17B vs 1.26B.
- Sony LJPEG: 56.5% `decode_interleaved`, 22.3% `add_prediction`, and 9.9%
  bit-reader lookahead. Sony's four-component MCUs miss Fornax's fast path,
  which only handles predictor 1 with one/two components. FlashRaw handles four
  values with register predictors, fused Huffman lookup, and parallel
  unstuffing.
- Uncompressed ARW/NEF: Sony and Nikon use `fs::read`, then generic tile code.
  A one-strip image creates one Rayon band, so 16 threads do not help; Nikon's
  unpacked-word loop is also serial. FlashRaw mmaps, row-parallel unpacks
  directly into the final buffer, and uses wider row kernels. On Sony ILCE-1,
  Fornax incurs 56,272 minor faults vs 2,522 and whole-probe instructions are
  nearly equal (1.856B vs 1.829B), but wall time is 7.9x slower. This is memory
  traffic, allocation residency, file copying, and lack of useful parallelism,
  not extra arithmetic.
- Leica DNG uncompressed exposes an API parity gap: Fornax applies
  `OpcodeList1` unconditionally; FlashRaw was run with opcodes disabled. The
  profile spends 37.4% in opcode sample conversion and 6.9% in opcode apply.
  The output happens to match, but the timing is not comparable.

## Ranked improvements

1. **Fix the two correctness paths first.** Correct 12-bit uncompressed ARW
   word-width selection and debug the Pentax two-component predictor-1 LJPEG
   path against FlashRaw/Rawler. Risk of shipping wrong pixels otherwise
   dominates performance work. Effort: low for Sony selection, medium for
   Pentax. No new unsafe, platform code, or SIMD.
2. **Make the generic uncompressed path row-parallel and direct.** When there is
   one strip/tile, split rows across Rayon; decode 16-bit words straight into
   the final buffer; mmap Sony/Nikon inputs instead of `fs::read`. Expected
   impact: multiple-fold on large uncompressed ARW/NEF and useful 16-thread
   scaling. Effort: medium, risk: medium; platform mmap already exists; no SIMD
   or new unsafe needed.
3. **Add a NEF Huffman fast row kernel.** Use a 13-bit fused Huffman/difference
   table and 64-bit bit windows; later add speculative parallel entropy. Expected
   impact: 2-5x at one thread and a large 16-thread gain. Effort: high, risk:
   medium; scalar first, no unsafe or platform code, SIMD not required.
4. **Add a specialized CRX Rice reader.** Implement a 64-bit cache with
   leading-zero code-word extraction before changing the wavelet pipeline.
   Expected impact: 1.5-3x at both thread counts. Effort: medium-high, risk:
   low if covered by exact-output tests; scalar, no unsafe or platform code.
5. **Extend lossless-JPEG fast rows to four components.** Reuse the fused
   13-bit table and register-resident predictor approach for Sony MCUs. Expected
   impact: 1.8-3x at one thread and better 16-thread efficiency. Effort:
   medium-high, risk: medium; optional SIMD only after scalar parity, no unsafe.
6. **Reduce output-allocation page faults.** Investigate why fresh Fornax
   decodes retain 20-60k minor faults while FlashRaw often has only 1-3k.
   Reuse or allocator-resident output buffers could materially help uncompressed
   paths. Effort: medium-high, risk: ownership/API changes; may need allocator
   unsafe/platform code, no SIMD.
7. **Expose benchmark-only opcode control for DNG.** This is measurement parity,
   not a speed optimization. Effort: low; risk: public/benchmark API surface;
   no unsafe, platform code, or SIMD.
8. **Optimize generic DNG LJPEG last.** Its 1T gap is 29% and scaling already
   matches FlashRaw; gains are smaller than NEF/CRX/uncompressed work. Scalar
   table/row work only; no unsafe, platform code, or SIMD initially.

The first performance-only project should be item 2. It is the simplest
high-impact change for a one-person maintainer and attacks both the largest
single-thread ratios and the near-total lack of scaling.

## 2026-11-16 low-effort follow-up

This follow-up corrected both previously reported mismatches and implemented
only the uncompressed-word part of improvement 2. It did not change Huffman,
CRX Rice, four-component LJPEG, DNG packed data, allocator selection, or SIMD.

- Sony ILCE-7RM4 DSC00395.ARW now hashes to
  `9533fd9cea0d92272364dff9c4b32b865f07fa8095e7cd78a481f90fd14fe310`.
  The declared 12 significant bits are stored in two-byte words; Fornax now
  selects the word path from `StripByteCounts`, while genuinely packed 12-bit
  strips retain the packed path.
- Pentax K-1 IMGP8555.DNG now hashes to
  `fb04a11babb78e2a11d721c0a6ba77f05a065c8ab14dd8c386f48780c321191e`.
  The LJPEG stream was correct. The DNG contains four equal-sized,
  full-resolution LJPEG IFDs; Fornax selected the last on a tie while FlashRaw
  and Rawler select the first. The decoder now keeps the first equal-sized raw
  IFD.
- The other 12 manifest hashes are unchanged and match the accepted baseline.

For uncompressed paths, the existing read-only mmap helper is now owned by
`TiffSource` and reused by DNG, Sony, and Nikon without a full-file `fs::read`.
A shared 16-bit-word unpack validates strip bounds once, writes directly into
the final output buffer, processes rows serially for one thread or tiny
images, and otherwise splits contiguous row bands across Rayon. Sony
uncompressed and Nikon word layouts both use it; Nikon applies its existing
12/14-bit mask and byte order. DNG 16-bit uncompressed reaches the same helper,
but Leica timing remains dominated by Fornax's non-equivalent `OpcodeList1`
work and is not compared to FlashRaw.

Method for the new numbers: the same host and optimized bench profile, warm
page cache, five requested files, three independent process batches, two
warmups, five rounds per batch, median of round medians followed by median of
batch medians. `F` is Fornax, `FR` is scalar-CPU FlashRaw, both in
milliseconds. The complete batch values, throughput, faults, spread, and hashes
are in
[fornax-vs-flashraw-uncompressed-2026-11-16.csv](data/fornax-vs-flashraw-uncompressed-2026-11-16.csv).

| Sample | T | F before | F after | F speedup | FR now | F/FR now | F GB/s | F faults after | spread |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Sony ILCE-1 | 1 | 222.60 | 29.44 | 7.56x | 28.63 | 1.03x | 3.41 | 2,521 | 4.3% |
| Sony ILCE-1 | 16 | 216.15 | 31.16 | 6.94x | 14.02 | 2.22x | 3.22 | 2,537 | 8.4% |
| Nikon D500 | 1 | 113.33 | 35.89 | 3.16x | 13.48 | 2.66x | 1.16 | 10,895 | 2.9% |
| Nikon D500 | 16 | 108.95 | 6.27 | 17.38x | 6.55 | 0.96x | 6.66 | 10,896 | 6.4% |
| Nikon D850 | 1 | 242.98 | 76.58 | 3.17x | 25.99 | 2.95x | 1.20 | 23,801 | 2.4% |
| Nikon D850 | 16 | 238.91 | 11.91 | 20.06x | 13.19 | 0.90x | 7.68 | 23,802 | 2.0% |
| Sigma fp | 1 | 0.94 | 0.98 | 0.96x | 1.18 | 0.83x | 4.31 | 50 | 19.5% |
| Sigma fp | 16 | 0.92 | 1.09 | 0.84x | 0.91 | 1.20x | 3.85 | 50 | 14.1% |
| Leica SL2 | 1 | 222.92 | 234.46 | 0.95x | 37.93 | n/c | 0.41 | 7,067 | 1.7% |
| Leica SL2 | 16 | 263.29 | 273.77 | 0.96x | 15.39 | n/c | 0.35 | 6,109 | 1.8% |

Sony at one thread is now within FlashRaw's batch spread. Nikon 16-thread
throughput slightly exceeds FlashRaw on these two runs. Sony's 16-thread result
is memory-bound and did not scale beyond its one-thread time; row-band
parallelism did not recover that after the copy was removed. Sigma's packed
12-bit path was intentionally untouched and its sub-millisecond deltas are
within noisy to 19% spread. The remaining clear gap is Sony 16-thread
throughput at 2.22x, not another copy or serial-strip defect in the measured
word paths.
