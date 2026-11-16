# DNG Decode Remaining Performance Gap

This note records the remaining DNG decode performance gap after the
[DNG decode performance work](../../archive/plans/perf-dng-decode.md). It is a report,
not an implementation plan. The acceptance target was to get the integer DNG
paths close to Rawler 0.8.0; FlashRaw 0.2.0 remains an upper reference.

The kept/reverted experiment history is consolidated in
[the DNG performance experiment log](dng-experiments-log.md). Reproduction
commands, input hashes, probe locations, and open items are in
[the DNG performance resume notes](dng-gap-profiles/RESUME.md).

## Result

The integer DNG target is met. Fornax is at parity with or faster than Rawler
on every integer case measured after the optimization work. At the PR #736
portable baseline, Fornax was faster than FlashRaw on the Sigma fp 12-bit
uncompressed case and slower on 5D3 uncompressed, floating-point Deflate, and
lossless JPEG. The restored Linux output policy returns 5D3 uncompressed to
FlashRaw parity and materially narrows the other gaps.

A follow-up measurement retained one additional compute optimization: the hot
predictor-1 path now buffers error state instead of constructing and propagating
a `Result` for every sample. It closes about one quarter of the remaining LJPEG
time on the three measured DNGs. PR #734 later removed the first-touch policy
from `fornax-codec`; PR #738 restored it as one platform-specific module with a
default-on Linux feature while leaving the decode API and scalar compute paths
unchanged.

PR #736 portable-baseline single-thread end-to-end results:

| File | Variant | Fornax | Rawler | FlashRaw | Fastest | Fornax/FlashRaw | Fornax/Rawler |
| --- | --- | ---: | ---: | ---: | --- | ---: | ---: |
| 5D3 | U16 uncompressed strip | 44.38 ms | 53.5 ms | 16.17 ms | FlashRaw | 2.75x | 0.83x |
| Sigma fp 4K | 12-bit uncompressed strip | 5.50 ms | 5.79 ms | 10.46 ms | Fornax | 0.53x | 0.95x |
| 5D3 | U16 LJPEG tiles | 197.47 ms | 272.1 ms | 92.80 ms | FlashRaw | 2.13x | 0.73x |
| Pentax K-5 | 14-bit LJPEG strip | not re-run | 168.5 ms | 77.0 ms | FlashRaw | n/m | n/m |
| iPhone SE | U16 LJPEG tiles | 72.83 ms | 80.8 ms | 41.28 ms | FlashRaw | 1.76x | 0.90x |
| Pixel 8 Pro | U16 LJPEG tiles | 432.65 ms | 546.4 ms | 195.49 ms | FlashRaw | 2.21x | 0.79x |
| 5D3 HDR | FP16 predictor Deflate | 236.45 ms | 454.2 ms | 189.03 ms | FlashRaw | 1.25x | 0.52x |
| 5D3 HDR | FP32 predictor Deflate | 397.19 ms | 766.1 ms | 322.33 ms | FlashRaw | 1.23x | 0.52x |

The timings are host-specific observations from the investigation environment:
WSL2 Debian, AMD Ryzen 7 5800H, release mode, page-cache-hot input, and one
worker for all decoders. Kernel `6.6.87.1-microsoft-standard-WSL2`, glibc
`2.41-12+deb13u3`, THP `[madvise]`, and THP defrag `[madvise]`. They are not
portable performance guarantees.

These baseline Fornax and FlashRaw values use three fresh, interleaved processes
per case with five measured rounds after one warmup; Sigma fp used fifteen
rounds after two warmups in the earlier run. Rawler values remain from the
original report because only the Fornax/FlashRaw ratios were re-run. Pentax K-5
is pointer-only in the current corpus checkout and Fornax has no Pentax
container decoder, so it could not be re-run through the three-way gate.

## Linux Large-Page Allocation

The allocation policy is platform-specific by design. Linux builds with the
default-on `large-page-hint` feature use natural-alignment
`alloc_zeroed(Layout::array::<T>(len))` for final outputs of at least 8 MiB,
advise the 2 MiB-aligned interior with `MADV_HUGEPAGE`, and adopt the block into
the same owned `Vec<T>` returned by the public API. Windows, macOS, Miri, feature
off, and smaller outputs take the plain zeroed-`Vec` path. The only new target
dependency is optional `libc` under
`[target.'cfg(target_os = "linux")'.dependencies]`.

The implementation and all unsafe code live in the independent
`fornax-large-alloc` crate,
[`large_alloc.rs`](../../../crates/fornax-large-alloc/src/large_alloc.rs). Final
decode outputs are allocated through that crate from
[`samples.rs`](../../../crates/fornax-codec/src/samples.rs),
[`deflate.rs`](../../../crates/fornax-codec/src/deflate.rs),
[`ljpeg.rs`](../../../crates/fornax-codec/src/ljpeg.rs),
[`jpeg.rs`](../../../crates/fornax-codec/src/jpeg.rs), and
[`jpegxl.rs`](../../../crates/fornax-codec/src/jpegxl.rs). LJPEG's <=8-bit final
U8 conversion is also routed through the module, closing a path missed by the
earlier policy. The module has two production unsafe blocks: raw allocation plus
Vec adoption, and `madvise`. Counting-allocator test code is confined to the same
file.

This table compares the exact PR #736 baseline with the restored module on the
same host session. Values are medians over three fresh processes x five rounds;
min-max ranges, fault counts, and the feature-off fallback are in
[the large-page allocation evidence](dng-gap-profiles/large-page-allocation.txt).

| File | Threads | Plain `Vec` baseline | With module | Change | Same-session FlashRaw | With/FlashRaw |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 5D3 uncompressed | 1 | 47.357 ms | 18.400 ms | -61.1% | 18.251 ms | 1.008x |
| 5D3 uncompressed | 15 | 47.036 ms | 18.070 ms | -61.6% | 18.411 ms | 0.982x |
| Sigma fp | 1 | 5.794 ms | 5.635 ms | -2.7% | 11.118 ms | 0.507x |
| Sigma fp | 15 | 5.850 ms | 5.742 ms | -1.8% | 11.438 ms | 0.502x |
| 5D3 LJPEG | 1 | 177.101 ms | 143.696 ms | -18.9% | 94.883 ms | 1.514x |
| 5D3 LJPEG | 15 | 31.790 ms | 30.092 ms | -5.3% | 93.804 ms | 0.321x |
| Pixel 8 LJPEG | 1 | 363.639 ms | 288.105 ms | -20.8% | 191.054 ms | 1.508x |
| Pixel 8 LJPEG | 15 | 61.866 ms | 54.165 ms | -12.4% | 190.613 ms | 0.284x |
| iPhone SE LJPEG | 1 | 61.711 ms | 61.455 ms | -0.4% | 41.161 ms | 1.493x |
| iPhone SE LJPEG | 15 | 13.836 ms | 13.307 ms | -3.8% | 41.419 ms | 0.321x |
| 5D3 FP16 Deflate | 1 | 239.522 ms | 180.030 ms | -24.8% | 189.690 ms | 0.949x |
| 5D3 FP16 Deflate | 15 | 54.220 ms | 44.281 ms | -18.3% | 190.327 ms | 0.233x |
| 5D3 FP32 Deflate | 1 | 401.427 ms | 340.140 ms | -15.3% | 328.783 ms | 1.035x |
| 5D3 FP32 Deflate | 15 | 66.707 ms | 60.219 ms | -9.7% | 334.123 ms | 0.180x |

The 15T FlashRaw wall-time behavior remains anomalous on this WSL host; ratios
are same-session but should not be read as cross-host speed guarantees. All
candidate medians are neutral or faster and no range shows a repeatable
regression.

The requested suggested 20% relative gate is not met by FP32 Deflate. The
module is retained because 5D3 uncompressed is unambiguous, FP32 still improves
by 61.3 ms at 1T and 6.5 ms at 15T with disjoint ranges, its same-session
FlashRaw ratio improves from 1.221x to 1.035x at 1T, and the 1T absolute result
matches the approximately 60 ms win in the earlier policy evidence. No measured
case is slower beyond process noise.

For the real 5D3 files, `AnonHugePages` changes from 0 to 43,008 KiB for
uncompressed and LJPEG and to 88,064 KiB for FP32. Fresh-process one-thread
minor faults fall from 12,750 to 2,021, 40,091 to 29,360, and 24,316 to 2,342,
respectively. A probe with unchanged faults or zero huge pages was rejected.

The host does not allow changing
`/sys/kernel/mm/transparent_hugepage/enabled` (non-root and `no_new_privs`), so
a literal THP `never` run could not be performed. The same candidate built with
`large-page-hint` disabled is the fallback proxy: every median stays within
2.5% of the PR #736 baseline except the noisy iPhone 15T case, and all ranges
overlap.

Windows MSVC and Apple arm64 checks pass with default features, with
`large-page-hint` alone, and with every pure-Rust codec feature. `libc` is absent
from both target dependency trees. `--all-features` reaches the pre-existing
`zstd-sys` cross-C compiler limitation. Miri is unavailable on the installed
nightly toolchain. The numbers above are Linux-only and unmeasured on Windows
or macOS.

### Maintaining this module

Re-check these invariants when Rust, glibc, or the kernel changes:

- `Layout::array::<T>(len)` must continue to describe exactly the Vec length and
  natural alignment. Never introduce over-alignment, because that causes the
  allocator to initialize before `madvise` and disables the first-touch win.
- `alloc_zeroed` and `Vec::from_raw_parts` must use the same global allocator,
  `capacity == len`, `T: Pod`, and matched drop layout.
- Keep the 8 MiB threshold and the 2 MiB page-aligned-interior calculation
  together; benchmark again before changing either.
- Re-run the `AnonHugePages` check and require changed minor-fault counts. A
  timing without a nonzero hint or fault change is not evidence that the policy
  engaged.

## Portable Allocation Regression (Historical Baseline)

At the PR #734 state, removing the platform allocator moved Fornax instruction
counts by less than 0.01% but restored first-touch page faults. The full
with/without-policy table, same-session FlashRaw values, fault counts, and
equality status are in
[the portable allocation regression](dng-gap-profiles/portable-allocation-regression.txt).

The material regressions are:

| File | Threads | With policy | Plain `Vec` | Change | FlashRaw ratio after |
| --- | ---: | ---: | ---: | ---: | ---: |
| 5D3 uncompressed | 1 | 17.314 ms | 44.383 ms | +156.33% | 2.745x |
| 5D3 uncompressed | 15 | 16.521 ms | 44.299 ms | +168.14% | 4.179x |
| 5D3 LJPEG | 1 | 169.048 ms | 197.471 ms | +16.81% | 2.128x |
| 5D3 LJPEG | 15 | 34.908 ms | 38.150 ms | +9.29% | 1.754x |
| Pixel 8 Pro LJPEG | 1 | 354.032 ms | 432.649 ms | +22.21% | 2.213x |
| Pixel 8 Pro LJPEG | 15 | 68.159 ms | 77.422 ms | +13.59% | 1.952x |
| 5D3 FP16 Deflate | 1 | 179.249 ms | 236.454 ms | +31.91% | 1.251x |
| 5D3 FP16 Deflate | 15 | 44.488 ms | 54.565 ms | +22.65% | 1.045x |
| 5D3 FP32 Deflate | 1 | 341.264 ms | 397.185 ms | +16.39% | 1.232x |
| 5D3 FP32 Deflate | 15 | 59.933 ms | 67.891 ms | +13.28% | 1.060x |

That portable baseline lost FlashRaw parity on 5D3 uncompressed, FP16 Deflate,
FP32 Deflate, and Pixel 8 all-thread LJPEG relative to the policy state. Sigma
fp remained faster than FlashRaw at one thread and effectively at parity at 15
threads. iPhone SE wall time is dominated by scalar LJPEG work; removing the
policy did not change its one-thread median outside noise.

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

Lossless JPEG remains the largest remaining gap. On the portable scalar state,
Fornax is 2.13x FlashRaw on 5D3, 2.21x on Pixel 8, and 1.76x on iPhone SE.
Fornax is faster than Rawler on the three re-measured cases for which Rawler
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

A historical follow-up found that LJPEG allocated its full-image U16 buffer
directly with `vec![0_u16; ...]`, bypassing the then-current large-integer
policy. Routing that allocation through the shared helper reduced 5D3 from
210.5 ms to 170.5 ms at one thread and from 41.8 ms to 38.2 ms at 15 threads.
Pixel 8 improved from 429.1 ms to 356.3 ms and from 82.2 ms to 70.6 ms. One-
thread minor faults fell from 40,162 to 29,429 on 5D3 and from 85,200 to 61,189
on Pixel 8. Those numbers describe the earlier Linux allocation iteration.
Details are in
[the LJPEG allocation follow-up](dng-gap-profiles/ljpeg-allocation-followup.txt).

A historical minor-fault call graph showed that, after using the removed output
policy, the remaining faults were no longer dominated by the full-image output.
Per-tile Huffman lookup allocation and initialization accounted for 86.9% of
5D3 one-thread fault samples and 80.1% on Pixel 8. The measured counters in that
state were 29,359 and 61,115 faults; at 15 threads they were 20,858 and 44,252.
At the PR #734 plain-`Vec` state the corresponding first-decode counts were
40,090 and 85,129 at one thread. Details and the fifteen-thread profile caveats are in
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

The 12-bit uncompressed path is faster than Rawler: Sigma fp is 5.50 ms versus
Rawler's 5.79 ms and FlashRaw's 10.46 ms. The grouped 12/14-bit unpacking and
native-endian U16 copy brought integer unpacking to Rawler parity. The 5D3
U16 output was 44.38 ms at the PR #734 portable baseline. With the restored
module it is 18.40 ms, approximately FlashRaw parity in the same session.

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

Under the removed policy, single-thread 5D3 uncompressed improved from 48.3 ms
to 18.0 ms and was approximately at parity with FlashRaw. The current portable
state is 44.38 ms versus FlashRaw's 16.17 ms, with 12,751 versus 1,805 minor
faults. Fornax still executes fewer instructions, about 7.14 million versus
FlashRaw's 7.90 million. The exact three-way dimensions, storage, bits, and
pixel gate passed.

The profile's 95.1% sample share for Fornax's native U16 tile-copy callback is
consistent with write/first-touch cost, not scalar unpacking. The codec already
uses a typed row copy. A reusable output buffer would remove most measured cost,
but it cannot preserve the current owned-return API and is no longer needed to
close the 5D3 gap.

## Deflate

Fornax beats Rawler by about 1.9x to 2.0x on both floating-point Deflate files.
After removing the platform output policy, Fornax is 1.25x FlashRaw on FP16 and
1.23x on FP32 at one thread, so the policy-era parity is not preserved by the
portable scalar state.

The current FP32 DWARF profile shows:

- Fornax: miniz fast inflate 58.8%, floating predictor reversal 23.2%, fused
  float-row decode 5.3%, match application 3.6%, Huffman initialization 3.3%,
  Adler-32 1.3%.
- FlashRaw: zlib-rs fast inflate 47.8%, fused float-row decode 27.4%, Adler-32
  8.7%, DNG float-row decode 4.8%, match copy 3.3%, Huffman initialization
  2.0%.

The fused predictor and float conversion work removed most of Fornax's separate
post-inflate passes, but inflate and byte/word reconstruction still dominate.
After removing the platform allocation policy, an isolated zlib-rs backend
retry improved FP16 by 5.8% at one thread but regressed it by 1.0% at 15
threads. FP32 improved by 10.5% and 5.1%. The backend was reverted because it
failed the both-thread-count keep rule. Fornax executes fewer instructions than
FlashRaw on FP32 Deflate (3.03B versus 3.84B per decode).

A minor-fault call-graph profile attributed 93.0% of FP32 faults to first writes
into the full F32 output. Inflate scratch accounted for about 1.1%, so
per-worker temporary reuse would not address the dominant fault source and was
not implemented. The profile is in
[the Deflate fault attribution](dng-gap-profiles/deflate-minor-faults.txt).

The historical floating-point policy result was invalidated by a probe that did not
report `AnonHugePages` or output fault counts. The corrected retry reported
88,064 KiB of `AnonHugePages` for the returned FP32 buffer, with one-thread
minor faults falling from 24,315 to 2,345 and instructions staying equal.
Interleaved medians improved FP16 by 25.4% and FP32 by 16.3% at one thread, and
by 18.4% and 9.4% at 15 threads. The policy was later removed for portability.
The PR #734 portable state reported 23,753 FP16 and 24,315 FP32 one-thread
faults. The restored module reports 2,342 FP32 faults and 88,064 KiB of
`AnonHugePages` in the fresh probe above.
Details are in
[the floating-point huge-page retry](dng-gap-profiles/float-hugepage-retry.txt).

## Portable Scalar Baseline at PR #734

PR #734 removed all platform allocation code from `fornax-codec`. That state
executes nearly identical instruction counts, but plain zeroed `Vec` allocation
restores first-touch faults. PR #738 restored the bounded Linux allocation
module; the scalar experiments and decisions below remain from the portable
work. Detailed historical with/without-policy wall, fault, and FlashRaw values
are in
[the portable allocation regression](dng-gap-profiles/portable-allocation-regression.txt).

The portable LJPEG experiments were isolated:

| Change | 1T result | 15T result | Decision |
| --- | ---: | ---: | --- |
| 12-bit fast table | mixed synthetic; noisy 2C +21% | not retained | Reverted |
| 11-bit fast table | mixed synthetic; noisy 2C +23% | not retained | Reverted |
| Canonical long-code path | 5D3 +1.2%; Pixel -5.7% | 5D3 +8.6%; Pixel +1.2% | Reverted |
| Shared `Arc` Huffman tables | 5D3 +21%; Pixel -1.2% | 5D3 +15.7%; Pixel +4.9% | Reverted |
| Fused scalar direct-band kernel | 5D3 +1.6%; iPhone +11.4%; Pixel -3.7% | mixed; no consistent win | Reverted |
| Fat LTO | 5D3 +6.5%; Pixel +0.7% | 5D3 +9.6%; Pixel +8.0% | Reverted |

The canonical path reduced faults but raised 5D3 instructions from 100.66 to
105.22 per sample. The fused kernel raised them to 102.31 on 5D3 and 94.30 on
Pixel 8. Its 130 ms target was 34.2% below the portable 197.471 ms baseline; it
missed by more than 35 percentage points. Exact data are in
[the portable scalar follow-up](dng-gap-profiles/portable-scalar.txt),
[the Huffman experiments](dng-gap-profiles/ljpeg-table-experiments.txt), and
[the caller-owned output prototype](dng-gap-profiles/ljpeg-reused-output.txt).
The fused-kernel rejection record remains on the separate
`chore/portable-ljpeg-fused-kernel` branch.

Caller-owned output reuse saved only 1.6-5.4% on synthetic tile loops and 1.6%
on a 5D3-sized full-output loop. It does not justify an additive public
decode-into API or its partial-write semantics.

Fornax scales 5.18x on 5D3 LJPEG, 3.68x on iPhone SE, and 5.59x on Pixel 8.
FlashRaw scales 4.27x, 5.08x, and 4.93x respectively. On 5D3 and Pixel the
scaling is similar enough that tile scheduling is not the primary gap. Current
LJPEG first-decode faults were 40,090/32,324 on 5D3 and 85,129/73,879 on Pixel
8 at one/fifteen threads in the portable baseline. Shared tables reduced faults
but not wall time.

## Historical Multi-thread Scaling

Fornax and FlashRaw were both run with all 15 visible workers on the tiled
lossless-JPEG and Deflate files. This table describes the removed Linux policy
state. Portable-state values are in
[the allocation regression](dng-gap-profiles/portable-allocation-regression.txt).
Values are medians from three fresh processes.

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

- The compute experiments and profiles span several allocation states. They
  remain valid for scalar work, but their wall-time and fault counters describe
  the policy named by each artifact.
- The final LJPEG state has a fresh post-C DWARF profile plus
  instruction/branch/fault counters. The fused-kernel prototype was rejected
  rather than retained. Remaining faults are attributed to per-tile Huffman
  table construction; a reused-buffer prototype reduced faults but not wall
  time and was reverted.
- Pentax K-5 could not be re-measured: its corpus file is pointer-only in this
  checkout and Fornax has no Pentax container decoder.
- The corrected float allocation probe confirmed 86 MiB of `AnonHugePages` and
  a 90-92% one-thread fault reduction. The current module reproduces the fault
  reduction; inflate scratch remains too small to justify a reuse prototype.
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
- The current large-page timings, AnonHugePages values, faults, fallback run,
  and equality status are in
  [large-page-allocation.txt](dng-gap-profiles/large-page-allocation.txt).
  PR #734 portable timings and fault evidence are in
  [portable-allocation-regression.txt](dng-gap-profiles/portable-allocation-regression.txt).
  Historical policy-era ratios are in
  [same-session-ratios.txt](dng-gap-profiles/same-session-ratios.txt),
  [ljpeg-remaining-faults.txt](dng-gap-profiles/ljpeg-remaining-faults.txt), and
  [float-hugepage-retry.txt](dng-gap-profiles/float-hugepage-retry.txt).
- The huge-page allocation path is Linux-only, lives in one module, and compiles
  to the plain-`Vec` path elsewhere. No Windows or macOS performance claim is
  made.

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

The follow-up attributed the remaining LJPEG faults to per-tile Huffman
construction and corrected the floating-point first-touch experiment. A later
portable pass removed the shared Linux allocator, its unsafe ownership transfer,
its allocator-only tests, and the `libc` dependency. PR #738 restored that policy
in `large_alloc.rs`, which owns all platform code and the two production unsafe
blocks. Per-worker Huffman buffer reuse reduced faults but did not improve wall
time and was reverted. Reusable unstuffing remained slower on noisy scans, and
both predictor changes regressed real 5D3 timing.

## Portable scalar LJPEG follow-up

Exact Callgrind attribution replaced sampled branch percentages. The retained
portable scalar changes are:

- truncating reconstructed samples without `u16::try_from` panic paths;
- component-local predictors and chunked row writes;
- a fixed-size 13-bit fast table;
- checked word refill with a cold stuffing/marker fallback;
- one-word packed fast entries.

On 5D3, exact whole-decode instructions fall from 2,564,082,227 to
1,551,617,888 (-39.48%). Native instructions/sample are 57.33 on 5D3,
53.06 on Pixel 8, and 50.56 on iPhone SE, or 1.41x, 1.38x, and 1.42x
same-session FlashRaw. Row-edge peeling was measured and reverted because it
regressed real-file instruction counts.

Wall time improves by 12.1-29.7%, not 39.48%. Cycles/sample fall from
29.055 to 24.361 on 5D3 1T, 27.636 to 23.064 on Pixel 8 1T, and 24.088
to 20.135 on iPhone SE 1T. FlashRaw is at 12.8-15.2 cycles/sample, so the
remaining gap is dominated by the serial Huffman lookup/shift dependency
chain rather than surplus retired instructions.

The full exact tables, per-step before/after results, register-pressure audit,
and rejected-change decisions are in the
[portable LJPEG follow-up index](dng-gap-profiles/README.md).

A subsequent critical-path pass replaced the 13-bit fused difference table
with a compact 10-bit canonical code table and branchless amplitude/sign
extension. On 5D3 1T, cycles/sample fall from 23.917 to 20.648 and wall time
from 175.302 ms to 157.404 ms, versus 14.604 cycles/sample and 94.130 ms for
FlashRaw in the same session. Pixel 8 1T wall time falls from 360.765 ms to
331.464 ms. The change retires more instructions, 79.581/sample, but cuts L1D
load misses from 0.96167 to 0.06010/sample and raises IPC from 2.397 to 3.854.

A reader-local rewrite removed the cache stack spill and reduced
instructions/sample to 48.126, but raised 1T cycles to 24.809, so it was
reverted. Unconditional refill cut branch misses/sample from 0.10636 to
0.01814 but raised cycles to 23.330 and was also reverted. The compact table
is not at exact FlashRaw parity: its serial shift/table/shift chain remains,
and the 1T cycle and wall ratios are 1.41x and 1.67x. Interleaving two tile
chains in one scalar loop is a reasonable separate design proposal, but no
such change was implemented here.

The merged #737/#738 baseline adds the materialized Pentax K-5 single-strip
case. At 1T it takes 93.2 ms and at 15T 92.4 ms, so tile-level parallelism
cannot help it. A fixed 12-bit packed-`u32` Huffman table reduced instructions
and improved tiled LJPEG, but raised Pentax to 106.9 ms and was reverted.
Component-pair refill raised real LJPEG instructions/sample from about 79.6 to
98.2 and wall time from 130.6 to 144.5 ms on 5D3 1T, so it was also reverted.
The compact 10-bit table and existing 32-bit refill remain. Single-scan
parallelism is stopped at a
[written boundary-design gate](restart-free-single-scan-parallelism-design.md);
the complete baseline and counter tables are in
[the FlashRaw CPU follow-up evidence](dng-gap-profiles/flashraw-cpu-followup.txt).

## Sources

- FlashRaw 0.2.0:
  [crate](https://crates.io/crates/flashraw/0.2.0),
  [source](https://docs.rs/crate/flashraw/0.2.0/source/), MIT OR Apache-2.0.
- Rawler 0.8.0:
  [crate](https://crates.io/crates/rawler/0.8.0),
  [source](https://docs.rs/crate/rawler/0.8.0/source/), LGPL-2.1.
- Historical Fornax measurements and profile notes:
  [DNG decode performance notes](../../archive/reference/decoders/dng-performance.md).
- Counter, timing, synthetic-LJPEG, and DWARF call-graph artifacts:
  [DNG gap profile artifacts](dng-gap-profiles/profiles.txt),
  [LJPEG hypotheses](dng-gap-profiles/ljpeg.txt),
  [uncompressed allocation probe](dng-gap-profiles/uncompressed.txt),
  [same-session ratios](dng-gap-profiles/same-session-ratios.txt).
- Follow-up fault and allocation evidence:
  [LJPEG remaining faults](dng-gap-profiles/ljpeg-remaining-faults.txt),
  [floating-point huge-page retry](dng-gap-profiles/float-hugepage-retry.txt),
  [current predictor instruction profile](dng-gap-profiles/ljpeg-instruction-followup.txt),
  [current large-page allocation](dng-gap-profiles/large-page-allocation.txt).
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
  [`large_alloc.rs`](../../../crates/fornax-large-alloc/src/large_alloc.rs),
  [`ljpeg.rs`](../../../crates/fornax-codec/src/ljpeg.rs),
  [`samples.rs`](../../../crates/fornax-codec/src/samples.rs),
  [`deflate.rs`](../../../crates/fornax-codec/src/deflate.rs).
