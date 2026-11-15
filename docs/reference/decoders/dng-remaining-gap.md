# DNG Decode Remaining Performance Gap

This note records the remaining DNG decode performance gap after the
[DNG decode performance work](../../plans/perf-dng-decode.md). It is a report,
not an implementation plan. The acceptance target was to get the integer DNG
paths close to Rawler 0.8.0; FlashRaw 0.2.0 remains an upper reference.

## Result

The integer DNG target is met. Fornax is at parity with or faster than Rawler
on every integer case measured after the optimization work. Fornax is at parity
with FlashRaw on 5D3 16-bit uncompressed and FP32 Deflate, faster on FP16
Deflate and the Sigma fp 12-bit uncompressed case, and remains slower on
lossless JPEG.

A follow-up measurement retained one additional LJPEG optimization: the hot
predictor-1 path now buffers error state instead of constructing and propagating
a `Result` for every sample. It closes about one quarter of the remaining LJPEG
time on the three measured DNGs. A later corrected first-touch probe invalidated
the earlier huge-page experiment and retained a Linux-only natural-align
`alloc_zeroed` plus `MADV_HUGEPAGE` policy for large output buffers. That policy
brings 5D3 uncompressed to parity with FlashRaw and was then applied to the
separately allocated LJPEG full-image output. A corrected floating-point retry
then showed a real first-touch gain, so the shared policy now covers every large
full-image output rather than only integer buffers.

Single-thread end-to-end results:

| File | Variant | Fornax | Rawler | FlashRaw | Fastest | Fornax/FlashRaw | Fornax/Rawler |
| --- | --- | ---: | ---: | ---: | --- | ---: | ---: |
| 5D3 | U16 uncompressed strip | 18.0 ms | 53.5 ms | 18.3 ms | parity | 0.98x | 0.34x |
| Sigma fp 4K | 12-bit uncompressed strip | 5.47 ms | 5.79 ms | 10.19 ms | Fornax | 0.54x | 0.95x |
| 5D3 | U16 LJPEG tiles | 166.3 ms | 272.1 ms | 94.8 ms | FlashRaw | 1.75x | 0.61x |
| Pentax K-5 | 14-bit LJPEG strip | not re-run | 168.5 ms | 77.0 ms | FlashRaw | n/m | n/m |
| iPhone SE | U16 LJPEG tiles | 73.1 ms | 80.8 ms | 41.3 ms | FlashRaw | 1.77x | 0.90x |
| Pixel 8 Pro | U16 LJPEG tiles | 346.8 ms | 546.4 ms | 192.9 ms | FlashRaw | 1.80x | 0.63x |
| 5D3 HDR | FP16 predictor Deflate | 178.5 ms | 454.2 ms | 193.3 ms | Fornax | 0.92x | 0.39x |
| 5D3 HDR | FP32 predictor Deflate | 338.4 ms | 766.1 ms | 334.3 ms | FlashRaw | 1.01x | 0.44x |

The timings are host-specific observations from the investigation environment:
WSL2 Debian, AMD Ryzen 7 5800H, release mode, page-cache-hot input, and one
worker for all decoders. Kernel `6.6.87.1-microsoft-standard-WSL2`, glibc
`2.41-12+deb13u3`, THP `[madvise]`, and THP defrag `[madvise]`. They are not
portable performance guarantees.

The current Fornax and FlashRaw values use three fresh, interleaved processes
per case with five measured rounds after one warmup; Sigma fp used fifteen
rounds after two warmups. Rawler values remain from the original report because
only the Fornax/FlashRaw ratios were re-run. Pentax K-5 is pointer-only in the
current corpus checkout and Fornax has no Pentax container decoder, so it could
not be re-run through the three-way gate.

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
- The headline measurements are single-threaded. A separate all-15-thread
  scaling comparison is included below.
- Historical DWARF call graphs are summarized in
  [DNG gap profile artifacts](dng-gap-profiles/profiles.txt). Current fault,
  allocation, predictor, and multi-thread evidence is linked in the relevant
  sections below.

## Lossless JPEG

Lossless JPEG remains the largest remaining gap, but the retained buffered-error
path reduces it to 1.75x FlashRaw on 5D3, 1.80x on Pixel 8, and 1.77x on iPhone
SE. Fornax is faster than Rawler on the three re-measured cases for which Rawler
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

A follow-up found that LJPEG allocated its full-image U16 buffer directly with
`vec![0_u16; ...]`, bypassing the large-integer policy. Routing that allocation
through the shared helper reduced 5D3 from 210.5 ms to 170.5 ms at one thread
and from 41.8 ms to 38.2 ms at 15 threads. Pixel 8 improved from 429.1 ms to
356.3 ms and from 82.2 ms to 70.6 ms. One-thread minor faults fell from 40,162
to 29,429 on 5D3 and from 85,200 to 61,189 on Pixel 8. Details are in
[the LJPEG allocation follow-up](dng-gap-profiles/ljpeg-allocation-followup.txt).

A subsequent minor-fault call graph showed that the remaining faults are no
longer dominated by the full-image output. Per-tile Huffman lookup allocation
and initialization account for 86.9% of 5D3 one-thread fault samples and 80.1%
on Pixel 8. The final measured counters are 29,359 and 61,115 faults,
respectively; at 15 threads they are 20,858 and 44,252. Details and the
fifteen-thread profile caveats are in
[the remaining-fault attribution](dng-gap-profiles/ljpeg-remaining-faults.txt).

Per-worker Huffman-buffer reuse was prototyped against that evidence. It cut the
5D3 warmup-plus-decode fault count from 30,683 to 3,033, but still rewrote both
lookup tables for every tile. Interleaved wall time was neutral on 5D3 and 1.9%
slower on Pixel 8 at 15 threads, so it was reverted.

Hypothesis A was retried with thread-local reusable unstuff scratch. Smooth
one-component scans improved, but noisy scans still regressed by 2.7-5.7%, and
minor faults were unchanged. The unstuff path was reverted; details are in
[the reusable-scratch retry](dng-gap-profiles/ljpeg-unstuff-retry.txt).

A fresh post-C DWARF profile now attributes 88.29% of 5D3 samples to the inlined
predictor-1 row/sample closure and 4.10% to `parse_jpeg`. The final path executes
about 100.67 instructions per output sample on 5D3. A benchmark-only fused
one/two-component kernel reduced one-component instructions/sample from 149.45
to 129.70, but its wall-time result did not remain stable. Two components
regressed from 140.56 to 158.89 instructions/sample and from about 500 ms to
530 ms on the 256-tile noisy scan. The prototype was removed. Details are in
[the post-C LJPEG profile](dng-gap-profiles/ljpeg-post-c.txt).

Direct instruction annotation placed the largest single site, the 13-bit fast
Huffman miss branch, at 25.2% of annotated instruction samples. Two isolated
changes failed to improve it: a 16-bit table increased table construction and
regressed 5D3 from 170.5 ms to 289.1 ms, while caching the symbol in a miss
entry regressed it to 216.8 ms. Both were reverted. FlashRaw remains at 40.70
instructions/sample versus about 100.67 for Fornax. See
[the predictor annotation follow-up](dng-gap-profiles/ljpeg-predictor-followup.txt)
and the current instruction rerun in
[the LJPEG instruction follow-up](dng-gap-profiles/ljpeg-instruction-followup.txt).

FlashRaw still fuses refill, Huffman lookup, difference reconstruction, and
prediction into a smaller instruction stream. Fornax's remaining cost is spread
across the row/sample loop and entropy refill; the isolated A, B, and D
experiments did not compose with the retained C path.

## Uncompressed

The 12-bit uncompressed path is no longer a problem: Sigma fp is 5.47 ms versus
FlashRaw's 10.19 ms and Rawler's 5.79 ms. The grouped 12/14-bit unpacking and
native-endian U16 copy brought integer unpacking to Rawler parity.

The earlier conclusion that huge pages did not help was based on an invalid
probe. The allocation was not 2 MiB-aligned and had already been zero-filled
before `MADV_HUGEPAGE`, so the huge-page and fresh variants both reported
172,123 faults. A corrected probe advises a 2 MiB-aligned natural allocation
before first write. It reports 43-45 MiB of `AnonHugePages` and changes the
fault count, which validates the experiment.

The pre-change 5D3 stage split summed to 42.33 ms in the instrumented process:

| Stage | Time |
| --- | ---: |
| TIFF parse and layout assembly | 1.81 ms |
| Source mmap faults | 4.45 ms |
| Output allocation and first touch | 27.52 ms |
| Pre-touched row copy | 4.90 ms |
| Residual | 3.65 ms |

The earlier stage table omitted source mapping faults and included an
out-of-band page-cache-hot `fs::read`. Most of its roughly 12 ms residual is
therefore 4.45 ms of source faults plus 3.65 ms of process/probe residual. The
rest is allocator and transparent-huge-page state varying between processes.

For the 23.4-million-sample U16 output, isolated allocation results were:

| Policy | Time after warmup | Five-round instruction total | Five-round faults |
| --- | ---: | ---: | ---: |
| Fresh `Vec<u16>` | 32-34 ms | 26,585,115 | 57,995 |
| `Vec::with_capacity` + `MADV_HUGEPAGE` | 12-15 ms | 39,322,960 | 4,347 |
| Natural `alloc_zeroed` + `MADV_HUGEPAGE` | 8.5-9.1 ms | 27,316,348 | 4,345 |
| Unchanged code with glibc hugetlb tunable | 8.0-8.7 ms | 28,318,743 | 4,342 |

The natural `alloc_zeroed` policy avoids the second zero-fill performed by the
safe `Vec::with_capacity` prototype and measured both materially faster and with
fewer instructions. It is now used only on Linux and for full-image output
buffers of at least 8 MiB. The later corrected float retry showed that the
earlier neutral/regression result was not valid evidence, so Deflate, generic
floating-point assembly, lossy JPEG, and JPEG XL now use the same boundary. The
owned-`Vec` API is unchanged.

The raw allocation uses `Layout::array::<T>(len)` exactly, transfers that block
to a `Vec` with capacity equal to length, reports allocation failure through
`handle_alloc_error`, and excludes `madvise` under Miri. A counting-allocator
test verifies both the raw allocation and its matching drop, plus the large F32
decode path.

Single-thread 5D3 uncompressed improved from 48.3 ms to 18.0 ms in the latest
same-session run; FlashRaw measured 18.3 ms. Final Fornax counters are about
6.55 million instructions and 1,586 faults per decode, close to FlashRaw's 7.27
million and 1,527. The exact three-way dimensions, storage, bits, and pixel gate
passed.

The profile's 95.1% sample share for Fornax's native U16 tile-copy callback is
consistent with write/first-touch cost, not scalar unpacking. The codec already
uses a typed row copy. A reusable output buffer would remove most measured cost,
but it cannot preserve the current owned-return API and is no longer needed to
close the 5D3 gap.

## Deflate

Fornax beats Rawler by about 2.3x to 2.5x on both floating-point Deflate files.
In the latest same-session comparison Fornax is 0.92x FlashRaw on FP16 and
1.01x on FP32, so both cases are at parity or faster.

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
3.84B per decode).

A minor-fault call-graph profile attributed 93.0% of FP32 faults to first writes
into the full F32 output. Inflate scratch accounted for about 1.1%, so
per-worker temporary reuse would not address the dominant fault source and was
not implemented. The profile is in
[the Deflate fault attribution](dng-gap-profiles/deflate-minor-faults.txt).

The old floating-point policy result was invalidated by a probe that did not
report `AnonHugePages` or output fault counts. The corrected retry reported
88,064 KiB of `AnonHugePages` for the returned FP32 buffer, with one-thread
minor faults falling from 24,315 to 2,345 and instructions staying equal.
Interleaved medians improved FP16 by 25.4% and FP32 by 16.3% at one thread, and
by 18.4% and 9.4% at 15 threads. The policy is retained. Exact full-image output
allocation sites now route through the shared allocator, and a
counting-allocator test covers a large F32 decode. Details are in
[the floating-point huge-page retry](dng-gap-profiles/float-hugepage-retry.txt).

## Multi-thread scaling

Fornax and FlashRaw were both run with all 15 visible workers on the tiled
lossless-JPEG and Deflate files. Values are medians from three fresh processes.

| File | Fornax 1T | Fornax 15T | FlashRaw 1T | FlashRaw 15T | Fornax/FlashRaw 15T |
| --- | ---: | ---: | ---: | ---: | ---: |
| 5D3 LJPEG tiles | 166.3 ms | 36.5 ms | 94.8 ms | 20.8 ms | 1.76x |
| Pixel 8 LJPEG tiles | 346.8 ms | 70.2 ms | 192.9 ms | 39.8 ms | 1.77x |
| 5D3 FP16 Deflate | 178.5 ms | 43.9 ms | 193.3 ms | 59.0 ms | 0.74x |
| 5D3 FP32 Deflate | 338.4 ms | 60.1 ms | 334.3 ms | 66.7 ms | 0.90x |

Fornax scales 4.55x and 4.94x on the LJPEG files, and FlashRaw scales 4.56x and
4.85x. On Deflate, Fornax scales 4.07x/5.63x versus FlashRaw's 3.28x/5.01x,
which puts Fornax ahead at all threads. These are host-specific observations.
Full data are in
[the same-session ratios](dng-gap-profiles/same-session-ratios.txt).

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

- The final LJPEG state has a fresh post-C DWARF profile plus
  instruction/branch/fault counters. The fused-kernel prototype was rejected
  rather than retained. Remaining faults are attributed to per-tile Huffman
  table construction; a reused-buffer prototype reduced faults but not wall
  time and was reverted.
- Pentax K-5 could not be re-measured: its corpus file is pointer-only in this
  checkout and Fornax has no Pentax container decoder.
- The corrected float allocation probe confirmed 86 MiB of `AnonHugePages` and
  a 90-92% one-thread fault reduction. Deflate output now uses the shared policy;
  inflate scratch remains too small to justify a reuse prototype.
- Reusable unstuffing scratch removed allocation from hypothesis A but still
  regressed noisy scans by 2-6%, so the implementation was reverted.
- The 5D3 fifteen-thread fault profile used period 4 and lost 117 of 20,776
  sampled events. Its per-call-site shares are therefore less precise than the
  one-thread profiles, although the table-fill dominated shape is unchanged.
- No cold-storage comparison was made.
- All reported ratios are single-file, page-cache-hot measurements and should
  not be aggregated into a corpus-wide speed claim. The scaling table uses all
  15 available workers; the headline table uses one worker.
- Corrected first-touch details are in
  [first-touch-corrected.txt](dng-gap-profiles/first-touch-corrected.txt), and
  the FlashRaw reference counters are in
  [flashraw-counters.txt](dng-gap-profiles/flashraw-counters.txt).
- Current same-session timings and fault evidence are in
  [same-session-ratios.txt](dng-gap-profiles/same-session-ratios.txt),
  [ljpeg-remaining-faults.txt](dng-gap-profiles/ljpeg-remaining-faults.txt), and
  [float-hugepage-retry.txt](dng-gap-profiles/float-hugepage-retry.txt).
- The huge-page allocation path is Linux-only. Windows allocator behavior and
  Windows huge-page performance were not measured, so no Windows optimization
  or performance claim is made.

## Encode

The uncompressed encoder now dispatches the `SampleBuffer` variant once, uses
a typed row writer for every tile, and parallelizes complete chunk payloads.
Synthetic encoding measurements used the same WSL2 Debian host as the decode
work: AMD Ryzen 7 5800H with 15 visible CPUs, release mode, hot in-memory input,
and no filesystem I/O in the timed region. Median and observed min-max ranges
come from 30 rounds after 10 warmups; the original baseline used 10 rounds
after 3 warmups. These numbers are host-specific.

| Case | Payload | Original | Typed serial | Final |
| --- | ---: | ---: | ---: | ---: |
| 5D3 U16, 256x256 tiles | 45,219,840 B, 345 chunks | 43.7 ms | 41.5 ms | 14.0 ms (13.4-14.5 ms) |
| 5D3 U16, 5760x16 strips | 44,236,800 B, 240 chunks | 39.2 ms | 38.4 ms | 12.4 ms (11.8-12.8 ms) |
| Small U16, 64x64 | 8,192 B, 16 chunks | 1.58 us | 0.94 us | 1.22 us (1.19-1.44 us) |

The indexed rayon change provided the large-input gain. Complete payloads are
dispatched by chunk index, so collection preserves row-major tile order. Small
payloads initially paid roughly 1 ms of rayon dispatch overhead. Measurements
around the crossover put an all-serial payload at 19.5 us for 512 KiB, while
1 MiB improved from 796 us on a single rayon worker to 646 us with the default
pool. The retained policy uses a serial fallback for one-chunk images and
payloads below 1 MiB, and rayon at or above that threshold.

The native-byte-order `bytemuck` row-copy hypothesis was reverted. Against the
generic loop it measured 13.9 versus 14.0 ms for 5D3 tiles, 12.4 versus 12.4 ms
for strips, and 1.21 versus 1.22 us for the small image, with overlapping
observed ranges. The differences were inside noise, so the simpler generic path
remains.

Per-chunk allocation was measurable in isolation: allocating the 345 tile
vectors took 11.3 ms, while one contiguous 45 MB allocation took 0.015 ms
before writes. A bench-only contiguous-buffer-plus-chunk-ranges prototype
completed a full tile encode in 12.9 ms versus 14.0 ms for the
`Vec<Vec<u8>>` API, and strips in 11.9 versus 12.4 ms. That 4-7% end-to-end
gain does not justify changing the public `EncodedChunks` representation
without a separate caller-owned-buffer API decision, so no production
allocation change is retained.

Before the refactor, the encoder output was snapshotted for all five sample
types, both byte orders, padded edge tiles, one-row, one-column, and
single-tile geometry. The final code matches all 40 original snapshots and
roundtrips each payload through `Geometry::new` and `decode_payloads`. The
seeded smooth/noisy fixture utility generates synthetic samples and
uncompressed payloads for codec tests and benchmarks; exact RAW inputs remain
in the Rawler and FlashRaw end-to-end comparisons.

## Follow-up Result

The follow-up hardened the shared allocator, attributed the remaining LJPEG
faults to per-tile Huffman construction, and corrected the floating-point
first-touch experiment. The shared float policy is retained, along with the
allocator soundness fixes and output-allocation coverage. Per-worker Huffman
buffer reuse reduced faults but did not improve wall time and was reverted.
Reusable unstuffing remained slower on noisy scans, and both predictor changes
regressed real 5D3 timing.

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
  [uncompressed allocation probe](dng-gap-profiles/uncompressed.txt),
  [same-session ratios](dng-gap-profiles/same-session-ratios.txt).
- Follow-up fault and allocation evidence:
  [LJPEG remaining faults](dng-gap-profiles/ljpeg-remaining-faults.txt),
  [floating-point huge-page retry](dng-gap-profiles/float-hugepage-retry.txt),
  [current predictor instruction profile](dng-gap-profiles/ljpeg-instruction-followup.txt).
- Benchmark adapter:
  [`flashraw_wrapper.rs`](../../../crates/fornax-raw-decoder/benches/flashraw_wrapper.rs).
- Exact-input profile probe:
  [`dng_gap_probe.rs`](../../../crates/fornax-raw-decoder/benches/dng_gap_probe.rs).
- Synthetic LJPEG probe:
  [`ljpeg_gap_probe.rs`](../../../crates/fornax-codec/benches/ljpeg_gap_probe.rs).
- Uncompressed encode probe:
  [`encode_probe.rs`](../../../crates/fornax-codec/benches/encode_probe.rs).
- Seeded synthetic fixture:
  [`synthetic.rs`](../../../crates/fornax-codec/tests/support/synthetic.rs).
- Fornax implementation:
  [`ljpeg.rs`](../../../crates/fornax-codec/src/ljpeg.rs),
  [`samples.rs`](../../../crates/fornax-codec/src/samples.rs),
  [`deflate.rs`](../../../crates/fornax-codec/src/deflate.rs).
