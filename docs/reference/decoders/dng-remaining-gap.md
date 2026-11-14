# DNG Decode Remaining Performance Gap

This note records the remaining DNG decode performance gap after the
[DNG decode performance work](../../plans/perf-dng-decode.md). It is a report,
not an implementation plan. The acceptance target was to get the integer DNG
paths close to Rawler 0.8.0; FlashRaw 0.2.0 remains an upper reference.

## Result

The integer DNG target is met. Fornax is at parity with or faster than Rawler
on every integer case measured after the optimization work. FlashRaw remains
faster on lossless JPEG, on the 5D3 16-bit uncompressed case, and on the two
floating-point Deflate cases. Fornax is faster than FlashRaw on the Sigma fp
12-bit uncompressed case.

Single-thread end-to-end results:

| File | Variant | Fornax | Rawler | FlashRaw | Fastest | Fornax/FlashRaw | Fornax/Rawler |
| --- | --- | ---: | ---: | ---: | --- | ---: | ---: |
| 5D3 | U16 uncompressed strip | 52.0 ms | 53.5 ms | 18.6 ms | FlashRaw | 2.80x | 0.97x |
| Sigma fp 4K | 12-bit uncompressed strip | 5.75 ms | 5.79 ms | 10.53 ms | Fornax | 0.55x | 0.99x |
| 5D3 | U16 LJPEG tiles | 266.0 ms | 272.1 ms | 95.8 ms | FlashRaw | 2.78x | 0.98x |
| Pentax K-5 | 14-bit LJPEG strip | 159.5 ms | 168.5 ms | 77.0 ms | FlashRaw | 2.07x | 0.95x |
| iPhone SE | U16 LJPEG tiles | 107.3 ms | 80.8 ms | 40.8 ms | FlashRaw | 2.63x | 1.33x |
| Pixel 8 Pro | U16 LJPEG tiles | 545.4 ms | 546.4 ms | 197.8 ms | FlashRaw | 2.76x | 1.00x |
| 5D3 HDR | FP16 predictor Deflate | 255.8 ms | 454.2 ms | 195.8 ms | Fornax | 1.31x | 0.56x |
| 5D3 HDR | FP32 predictor Deflate | 410.8 ms | 766.1 ms | 332.7 ms | Fornax | 1.23x | 0.54x |

The timings are host-specific observations from the investigation environment:
WSL2 Debian, AMD Ryzen 7 5800H, release mode, page-cache-hot input, and one
worker for all decoders. They are not portable performance guarantees.

## Comparison Scope

The headline comparison is Fornax `Decoder::decode(path).raw_mosaic()` against
Rawler `decode_file` and FlashRaw `decode`. All three include file opening,
metadata parsing, pixel decoding, and native U16 or F32 output allocation.
Crop and sample-type conversion are excluded.

FlashRaw is configured with `Backend::Cpu`, strict corruption handling,
`apply_opcodes = false`, `crop = false`, and `threads = Some(1)` in the
[benchmark adapter](../../../crates/fornax-raw-decoder/benches/flashraw_wrapper.rs).
The three-way equality gate checks exact dimensions, storage type, sample bits,
and pixel values before the ratios are trusted.

The comparison still has these caveats:

- Fornax retains a typed TIFF `Directory`, while the references normalize
  metadata into their own models. Fornax metadata work is included in the
  end-to-end result.
- FlashRaw maps the input lazily. Its decode time includes page faults as sensor
  data is touched.
- The measurements are single-threaded. Multi-thread scaling is not part of
  this report.
- No fresh post-optimization flamegraph was captured for every remaining case.
  The stage attribution below combines earlier profiles, instruction counters,
  final wall times, and source comparison.

## Lossless JPEG

Lossless JPEG is the largest remaining gap. Fornax is 2.07x to 2.78x slower than
FlashRaw, although it is within 1.33x of Rawler and beats Rawler on three of the
four files.

The pre-optimization profile attributed about 95% of Fornax's 5D3 LJPEG samples
to:

- `decode_scan_into`: 57.3%;
- `add_prediction`: 32.3%;
- `BitReader::peek`: 5.6%.

For the same workload, Fornax executed about 5.2x FlashRaw's instructions and
incurred about 20x its branch misses. The fused predictor-1 decoder removed much
of that cost, including full-tile staging and per-sample generic prediction, but
the final timings show that instruction and branch efficiency are still the
main explanatory family.

The concrete remaining differences, in likely order of importance, are:

1. **Entropy stream handling.** FlashRaw removes `0xff 00` byte stuffing from
   the scan once, then decodes with a plain MSB bit pump. Fornax checks marker
   bytes whenever its fast reader refills. The check is reduced to one 8-byte
   test on the common path, but it remains in every refill and has a
   byte-at-a-time fallback in
   [`fornax-codec/src/ljpeg.rs`](../../../crates/fornax-codec/src/ljpeg.rs).
2. **Component specialization.** FlashRaw has constant-specialized interleaved
   loops for one, two, three, and four components. It keeps predictors in
   registers and amortizes refills across samples. Fornax's fast path supports
   one and two components but still enumerates components dynamically and
   calculates an output index per sample.
3. **Per-sample error flow.** FlashRaw's hot decoder consumes a buffered
   Huffman entry without returning a `Result` for every sample and calls a cold
   path only for unusual codes. Fornax propagates `Result` from
   `decode_difference_fast` inside the sample loop.
4. **Lookup strategy.** Both implementations now have a fused
   code-plus-difference lookup. Fornax uses a 13-bit fast table; FlashRaw uses a
   12-bit table with branchless sign extension and a buffered error flag. Table
   width alone is not evidence of a large gap.
5. **Tile-local setup.** Every tile carries JPEG headers and Huffman tables.
   Fornax allocates and fills both a 16-bit symbol table and a 13-bit fused table
   for each table definition. FlashRaw attributed 3.2% of its profile to Huffman
   table construction. Reusing tile tables when definitions are identical is a
   possible target for high-tile-count files, but was not isolated.

The exact current split among these items has not been measured. A fresh
post-optimization profile of the 5D3 and Pixel 8 files is required before
assigning numeric shares to them.

## Uncompressed

The 12-bit uncompressed path is no longer a problem: Sigma fp is 5.75 ms versus
FlashRaw's 10.53 ms and Rawler's 5.79 ms. The grouped 12/14-bit unpacking and
native-endian U16 copy brought integer unpacking to Rawler parity.

The remaining 5D3 U16 case is 52.0 ms versus FlashRaw's 18.6 ms, despite Fornax
being slightly faster than Rawler. The codec's final U16 write is a typed row
copy, so the missing 2.8x factor is not explained by SIMD bit unpacking.

Two implementation differences are plausible contributors:

- Fornax allocates a zero-initialized ordinary `Vec<u16>` destination and then
  copies each row into it.
- FlashRaw allocates the destination with `alloc_zeroed`, applies Linux
  `MADV_HUGEPAGE` to large allocations, and directly unpacks rows into the final
  buffer. Its source notes that first-touch page faults dominate memory-bound
  U16 unpacking and that 2 MiB pages reduce fault counts substantially.

This does not yet prove that page-fault or allocation policy explains the full
33.4 ms difference. The next probe should collect a current profile and page
fault counters for the 5D3 uncompressed case.

## Deflate

Fornax beats Rawler by about 1.8x on both floating-point Deflate files and is
still 1.23x to 1.31x slower than FlashRaw.

The earlier FP32 profile showed similar proportions:

- Fornax: miniz inflate 48.8%, predictor reversal 25.3%, Deflate match
  application 3.4%, Huffman initialization 3.3%, float materialization 2.0%.
- FlashRaw: zlib-rs inflate 45.6%, fused float-row decode 28.6%, Adler-32 8.1%.

The fused predictor and float conversion work removed most of Fornax's separate
post-inflate passes, but inflate and byte/word reconstruction still dominate.
Replacing miniz with zlib-rs only improved the old Fornax FP32 result by about
2%, so a backend swap is not a credible standalone fix for the remaining gap.
Large-page allocation and first-touch behavior may also contribute, but that
hypothesis has not been isolated for Deflate.

## SIMD and Compiler Features

SIMD is not the explanation for these benchmark gaps:

- FlashRaw requests its scalar CPU backend in the benchmark adapter.
- `-C target-cpu=native` measured 475 ms for 5D3 LJPEG versus 459 ms generic,
  164 ms for uncompressed versus 145 ms generic, and 620 ms for FP32 Deflate
  versus 646 ms generic. The result was neutral or worse for the integer paths.
- The compiler may auto-vectorize scalar loops, but no explicit SIMD feature is
  required to explain the behavior, and enabling the native CPU target did not
  close the gap.

## Evidence Limits

- There is no post-Task-2 flamegraph for the final LJPEG implementation.
- The uncompressed 5D3 gap has not been separated into allocation, page fault,
  copy, and TIFF assembly costs.
- The Deflate gap has not been re-profiled after the fused predictor work.
- No multi-threaded or cold-storage comparison was made.
- All reported ratios are single-file, single-thread, page-cache-hot
  measurements and should not be aggregated into a corpus-wide speed claim.

## Next Probes

If further optimization is requested, the highest-information probes are:

1. Profile final Fornax and FlashRaw on 5D3 LJPEG, Pixel 8 LJPEG, 5D3
   uncompressed, and FP32 Deflate with DWARF symbols.
2. Measure an unstuffed-entropy LJPEG experiment independently from the
   component-specialized loop experiment.
3. Measure a predictor-1 kernel specialized for one and two components with
   buffered error handling.
4. Capture minor faults and page-fault time for the 5D3 uncompressed output
   allocation before considering allocation or large-page changes.

These are experiments, not approved implementation work. The current result
already satisfies the Rawler-relative acceptance target.

## Sources

- FlashRaw 0.2.0:
  [crate](https://crates.io/crates/flashraw/0.2.0),
  [source](https://docs.rs/crate/flashraw/0.2.0/source/), MIT OR Apache-2.0.
- Rawler 0.8.0:
  [crate](https://crates.io/crates/rawler/0.8.0),
  [source](https://docs.rs/crate/rawler/0.8.0/source/), LGPL-2.1.
- Fornax measurements and profile notes:
  [DNG decode performance notes](dng-performance.md).
- Benchmark adapter:
  [`flashraw_wrapper.rs`](../../../crates/fornax-raw-decoder/benches/flashraw_wrapper.rs).
- Fornax implementation:
  [`ljpeg.rs`](../../../crates/fornax-codec/src/ljpeg.rs),
  [`samples.rs`](../../../crates/fornax-codec/src/samples.rs),
  [`deflate.rs`](../../../crates/fornax-codec/src/deflate.rs).
