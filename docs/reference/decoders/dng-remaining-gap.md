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

A follow-up measurement retained one additional LJPEG optimization: the hot
predictor-1 path now buffers error state instead of constructing and propagating
a `Result` for every sample. It closes about one quarter of the remaining LJPEG
time on the three measured DNGs. No uncompressed allocation-policy change was
retained.

Single-thread end-to-end results:

| File | Variant | Fornax | Rawler | FlashRaw | Fastest | Fornax/FlashRaw | Fornax/Rawler |
| --- | --- | ---: | ---: | ---: | --- | ---: | ---: |
| 5D3 | U16 uncompressed strip | 52.0 ms | 53.5 ms | 18.6 ms | FlashRaw | 2.80x | 0.97x |
| Sigma fp 4K | 12-bit uncompressed strip | 5.75 ms | 5.79 ms | 10.53 ms | Fornax | 0.55x | 0.99x |
| 5D3 | U16 LJPEG tiles | 202.6 ms | 272.1 ms | 96.9 ms | FlashRaw | 2.09x | 0.74x |
| Pentax K-5 | 14-bit LJPEG strip | not re-run | 168.5 ms | 77.0 ms | FlashRaw | n/m | n/m |
| iPhone SE | U16 LJPEG tiles | 75.4 ms | 80.8 ms | 41.0 ms | FlashRaw | 1.84x | 0.93x |
| Pixel 8 Pro | U16 LJPEG tiles | 423.7 ms | 546.4 ms | 194.3 ms | FlashRaw | 2.18x | 0.78x |
| 5D3 HDR | FP16 predictor Deflate | 255.8 ms | 454.2 ms | 195.8 ms | Fornax | 1.31x | 0.56x |
| 5D3 HDR | FP32 predictor Deflate | 410.8 ms | 766.1 ms | 332.7 ms | Fornax | 1.23x | 0.54x |

The timings are host-specific observations from the investigation environment:
WSL2 Debian, AMD Ryzen 7 5800H, release mode, page-cache-hot input, and one
worker for all decoders. They are not portable performance guarantees.

The final LJPEG numbers use three fresh processes per case and decoder with five
interleaved rounds per process. The raw and FlashRaw values used for the old
unchanged cases remain from the original report. Pentax K-5 is pointer-only in
the current corpus checkout and Fornax has no Pentax container decoder, so it
could not be re-run through the three-way gate.

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
- DWARF call graphs, normalized instruction/branch/fault counters, and final
  wall times are recorded under
  [DNG gap profile artifacts](dng-gap-profiles/profiles.txt).

## Lossless JPEG

Lossless JPEG remains the largest remaining gap, but the retained buffered-error
path reduces it to 2.09x FlashRaw on 5D3, 2.18x on Pixel 8, and 1.84x on iPhone
SE. Fornax is faster than Rawler on the two re-measured cases for which Rawler
supports the input.

Fresh DWARF profiles of the pre-change 5D3 workload attributed Fornax's own
samples to:

- 38.3%: inlined row/sample sink closure;
- 28.6%: `HuffmanTable::decode_difference_fast`;
- 23.3%: `FastBitReader::fill_to`;
- 6.2%: `parse_jpeg`;
- 3.1%: fast row-decode closure.

FlashRaw's own samples were:

- 91.8%: fused `decode_row_pred1`;
- 3.2%: scan unstuffing;
- 2.8%: Huffman-table construction;
- 0.8%: DNG compressed-U16 row closure.

Pixel 8 had the same shape within one percentage point. Full call graphs and
folded stacks are under [DNG gap profile artifacts](dng-gap-profiles/profiles.txt).

The hypotheses were isolated against the same baseline:

1. **A, unstuff once.** It improved 5D3 by 2.35% and Pixel 8 by 3.77% when
   tested alone, but caused 12-16% noisy-scan regressions when combined with C.
   It was reverted.
2. **B, const-generic one/two-component loops.** It improved 5D3 by 9.40% and
   Pixel 8 by 10.38% when tested alone. In combination with C it was 1.4% and
   1.1% slower than C alone, at or within the larger spread, so it was reverted
   in favor of the simpler C-only result.
3. **C, buffered error flag.** It improved 5D3 by 24.37%, Pixel 8 by 24.30%,
   and iPhone SE by 29.7%, with synthetic noisy scans improving 7.6-8.8% and
   smooth scans improving 25-31%. It was retained.
4. **D, Huffman-table reuse.** It regressed 5D3 by 1.49%, beyond the baseline
   0.70% spread, without a distinguishable Pixel 8 gain. It was reverted.

The retained path removes per-sample `Result` construction and propagation.
Final counters execute 35.4% fewer instructions on 5D3 and 35.6% fewer on Pixel
8. Branch-miss counts are essentially unchanged (about +3%), so the gain is
instruction/control-flow reduction rather than a lower branch-miss rate.

FlashRaw still fuses refill, Huffman lookup, difference reconstruction, and
prediction into a smaller instruction stream. Fornax's remaining cost is spread
across the row/sample loop and entropy refill; the isolated A, B, and D
experiments did not compose with the retained C path.

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

The current probe separates the in-memory path into these measurements:

| Stage | Measurement |
| --- | --- |
| Page-cache-hot `fs::read` into an owned buffer | 45-62 ms, including the source copy |
| Open plus file mapping setup | about 0.05 ms; source faults remain lazy |
| TIFF parse and metadata/layout assembly | about 1.9-2.1 ms |
| Pre-touched row copy | 4.960 ms |
| Allocation and output first touch | about 32.5 ms |

The 23.4-million-sample output was then decoded into three destinations:

- fresh `Vec<u16>`: 37.433 ms total;
- pre-touched reused `Vec<u16>`: 4.960 ms;
- page-aligned `alloc_zeroed` plus `MADV_HUGEPAGE`: 40.212 ms total.

The fresh path incurred 172,123 process minor faults in the probe. The reused
path incurred 115,028, while the hugepage path still incurred 172,123. Hugepage
allocation therefore did not remove first-touch faults and was 2.8 ms slower
than a fresh vector. No allocation-policy change is retained.

The profile's 95.1% sample share for Fornax's native U16 tile-copy callback is
consistent with write/first-touch cost, not scalar unpacking. The codec already
uses a typed row copy. A reusable output buffer would remove most measured cost,
but it cannot preserve the current owned-return API; recommending it would be an
API redesign rather than a drop-in optimization.

## Deflate

Fornax beats Rawler by about 1.8x on both floating-point Deflate files and is
still 1.23x to 1.31x slower than FlashRaw.

The current FP32 DWARF profile shows:

- Fornax: miniz fast inflate 58.8%, floating predictor reversal 23.2%, fused
  float-row decode 5.3%, match application 3.6%, Huffman initialization 3.3%,
  Adler-32 1.3%.
- FlashRaw: zlib-rs fast inflate 47.8%, fused float-row decode 27.4%, Adler-32
  8.7%, DNG float-row decode 4.8%, match copy 3.3%, Huffman initialization
  2.0%.

The fused predictor and float conversion work removed most of Fornax's separate
post-inflate passes, but inflate and byte/word reconstruction still dominate.
Replacing miniz with zlib-rs only improved the old Fornax FP32 result by about
2%, so a backend swap is not a credible standalone fix for the remaining gap.
Fornax executes fewer instructions than FlashRaw on FP32 Deflate (3.03B versus
3.84B per decode) but incurs 13.3x its minor faults (23,787 versus 1,789).
Allocation/first-touch was not independently varied for Deflate in this task;
no Deflate implementation change is retained.

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

- The final LJPEG state has complete instruction/branch/fault counters but not
  a fresh post-change flamegraph.
- Pentax K-5 could not be re-measured: its corpus file is pointer-only in this
  checkout and Fornax has no Pentax container decoder.
- The Deflate profile is current, but its allocation/first-touch behavior was
  not independently varied and no Deflate change is included.
- No multi-threaded or cold-storage comparison was made.
- All reported ratios are single-file, single-thread, page-cache-hot
  measurements and should not be aggregated into a corpus-wide speed claim.

## Next Probes

If further work is requested, a post-C DWARF profile should first locate the
remaining Fornax instruction stream. Reusable caller-owned output buffers would
require an API design decision because the current owned-return contract cannot
reuse the destination without changing ownership. Deflate allocation/first-touch
would need a separate probe before any implementation change.

These are future experiments, not implied work in this report.

## Sources

- FlashRaw 0.2.0:
  [crate](https://crates.io/crates/flashraw/0.2.0),
  [source](https://docs.rs/crate/flashraw/0.2.0/source/), MIT OR Apache-2.0.
- Rawler 0.8.0:
  [crate](https://crates.io/crates/rawler/0.8.0),
  [source](https://docs.rs/crate/rawler/0.8.0/source/), LGPL-2.1.
- Fornax measurements and profile notes:
  [DNG decode performance notes](dng-performance.md).
- Counter, timing, synthetic-LJPEG, and DWARF call-graph artifacts:
  [DNG gap profile artifacts](dng-gap-profiles/profiles.txt),
  [LJPEG hypotheses](dng-gap-profiles/ljpeg.txt),
  [uncompressed allocation probe](dng-gap-profiles/uncompressed.txt).
- Benchmark adapter:
  [`flashraw_wrapper.rs`](../../../crates/fornax-raw-decoder/benches/flashraw_wrapper.rs).
- Exact-input profile probe:
  [`dng_gap_probe.rs`](../../../crates/fornax-raw-decoder/benches/dng_gap_probe.rs).
- Synthetic LJPEG probe:
  [`ljpeg_gap_probe.rs`](../../../crates/fornax-codec/benches/ljpeg_gap_probe.rs).
- Fornax implementation:
  [`ljpeg.rs`](../../../crates/fornax-codec/src/ljpeg.rs),
  [`samples.rs`](../../../crates/fornax-codec/src/samples.rs),
  [`deflate.rs`](../../../crates/fornax-codec/src/deflate.rs).
