> Status: archived historical benchmark report.
> Current document: [FlashRaw versus Fornax raw decode](../../../../reference/decoders/perf/flashraw-vs-fornax.md).
> Archived 2026-11-16; crop/cast and pipeline stages are no longer measured.

# FlashRaw 0.3.1 Re-measurement

This note records a lossless-only FlashRaw 0.2.0 to 0.3.1 comparison for DNG,
Sony ARW, Canon CR3, and Nikon NEF, including the Fornax decode-to-preprocess
path and the remaining optimization opportunities.

## Scope and conclusions

FlashRaw 0.3.1 is output-compatible with 0.2.0 on all 14 selected samples: both
the storage-native raw mosaic and the active-area `f32` crop matched bit for
bit. The adapter and its tests needed no source changes for the version bump.

The strongest 0.3.1 improvements are in compressed paths:

- Sony lossless JPEG decode improved by 29.6-29.8%.
- Nikon lossless Huffman decode improved by 26.8-28.5%.
- DNG lossless JPEG regressed by 12.1-12.2%, despite the release being
  described as an output-preserving rewrite. This is the main upstream
  regression to report.
- CR3 lossless CRX improved by 2.6-4.8%.
- Uncompressed results are mixed: DNG improved by 5.7-38.4%, ARW regressed by
  2.8-4.8%, and Nikon was within -0.2 to +2.3%.

The largest remaining in-tree cost is not codec time. The FlashRaw benchmark
adapter performs a scalar full-frame `u16`/`f32` conversion, crops that full
frame, and then shrinks the allocation. On large lossless files this stage was
about 53-64% of the combined decode -> crop/cast -> preprocess time; on large
uncompressed files it was about 62-89%. Preprocessing itself was only about
1-9% of the measured pipeline. The first optimization should therefore be a
direct active-area crop/cast, ideally fused with normalization.

No lossy mode was benchmarked, profiled, or optimized.

## Revisions and behavior changes

The prior pin was FlashRaw 0.2.0 (`dd782e5ccd9b9de7bd7fa42b562813e3de20722b`
in the published crate). The new pin is 0.3.1
(`61c32fb1abbc9e819084e01e62703ceb4c4998db` in the published crate; release tag
`v0.3.1` is `2994e8c550bec70ab78a938ab28a74d7762124f4`).

Pinned release material:

- [FlashRaw 0.3.1 release](https://github.com/ybouane/flashraw/releases/tag/v0.3.1)
- [FlashRaw changelog at v0.3.1](https://github.com/ybouane/flashraw/blob/v0.3.1/CHANGELOG.md)

Release 0.3.1 itself adds Nikon High Efficiency and High Efficiency star NEF
support. Those modes are explicitly outside this investigation. The
performance-relevant change is the 0.3.0 rewrite of the container decoders,
codecs, and camera database. Relevant changes include 64-bit bit pumps,
lookup-based Huffman decoding, SIMD packed unpacking, tile/strip/stripe
parallelism, zeroed output allocation, speculative parallel entropy decoding,
and parallel byte unstuffing for lossless JPEG.

The public `FlashRaw`, `DecodeOptions`, `RawFrame`, and `RawPixelData` surface
used by Fornax remains compatible. The 0.3.1 additions used by this review are
optional metadata accessors (`exif_bytes`, `xmp_bytes`); Fornax's raw benchmark
adapter does not call them.

## Sample matrix

The complete machine-readable matrix, including every checksum, is
[flashraw-0.3.1-matrix.csv](data/flashraw-0.3.1-matrix.csv). The benchmark
loader manifest is
[flashraw-0.3.1-bench-manifest.csv](data/flashraw-0.3.1-bench-manifest.csv).

| Brand | Model | Container | Mode | Bits | Resolution |
| --- | --- | --- | --- | ---: | ---: |
| Sigma | fp | DNG | uncompressed | 12 | 1936x1090 |
| Leica | SL2 | DNG | uncompressed | 14 | 8424x5632 |
| Pentax | K-1 | DNG | lossless JPEG | 14 | 7392x4950 |
| Google | Pixel 8 Pro | DNG | lossless JPEG | 16 | 8160x6144 |
| Sony | ILCE-7RM4 | ARW | uncompressed | 12 | 9600x6376 |
| Sony | ILCE-1 | ARW | uncompressed | 14 | 8672x5784 |
| Sony | ILCE-9M3 | ARW | lossless JPEG | 14 | 4096x3072 |
| Sony | ILCE-7RM5 | ARW | lossless JPEG | 14 | 9728x6656 |
| Canon | EOS R6 | CR3 | lossless CRX | 14 | 3584x2386 |
| Canon | EOS R5 Mark II | CR3 | lossless CRX | 14 | 8480x5650 |
| Nikon | D500 | NEF | uncompressed words | 14 | 5600x3728 |
| Nikon | D850 | NEF | uncompressed words | 14 | 8288x5520 |
| Nikon | Z5 II | NEF | lossless Huffman | 14 | 4000x2672 |
| Nikon | Z7 | NEF | lossless Huffman | 14 | 8288x5520 |

Coverage includes two uncompressed and two lossless samples for DNG, ARW, and
NEF, spanning multiple generations and resolutions. Canon CR3 has two lossless
samples but no uncompressed sample: the corpus and the CR3 family provide
lossless CRX or C-RAW/lossy data, not an uncompressed CRX mode.

The grouped exclusions are in
[flashraw-0.3.1-exclusions.csv](data/flashraw-0.3.1-exclusions.csv). Known lossy
rows were dropped: 115 Sony ARW2, 4 Sony ARW6, 1 DNG JPEG XL, and 2 DNG lossy
JPEG samples. Filename-identified C-RAW and Nikon High Efficiency samples were
also excluded. Remaining rows that the tracker could not classify as proven
lossless were excluded rather than assumed lossless.

## Method

Host:

- WSL2 kernel `6.6.87.1-microsoft-standard-WSL2`, Debian 13 userland.
- AMD Ryzen 7 5800H, 8 cores / 16 threads, 16 visible CPUs, 12 GiB RAM.
- Rust 1.100.0 (`0ef1bc386`, 2026-11-09), LLVM 23.1.5.
- Cargo 1.100.0 (`3acc79c0c`, 2026-11-05).
- Cargo bench profile: optimized release profile, default `opt-level=3`,
  `lto=off`, and `codegen-units=16`.

The 0.2.0 crate was no longer available from the live registry, so the baseline
used the cached published 0.2.0 packages whose SHA-256 values still matched the
committed `Cargo.lock`. The final upgrade resolves 0.3.1 from crates.io.

The benchmark used this fixed configuration:

- Fornax process metadata supplied source levels to both decoders.
- FlashRaw called `open_path` then `decode` with raw output, CPU scalar
  backend, strict corruption handling, crop disabled, opcodes disabled, and
  the stated worker count.
- Fornax end-to-end called `decode`, `cropped_mosaic::<f32>()`, then
  `Preprocessor`.
- Decode-only time includes container parsing and decoder construction.
- Pipeline time includes decode, active-area crop/cast to `f32`, and
  black/white preprocessing.
- Three independent baseline batches and three independent 0.3.1 batches were
  run. Each process performed two warmups and five measured rounds per stage.
- Reported values are the median of the three batch medians. Batch spread is
  `(max batch median - min batch median) / median`.
- Output equivalence hashes cover storage type, dimensions, and all sample
  bits. Cropped hashes cover active-area `f32` sample bits.

Representative command:

```bash
FORNAX_BENCH_MANIFEST=docs/reference/decoders/perf/data/flashraw-0.3.1-bench-manifest.csv \
FORNAX_BENCH_CORPUS=external/raw.pixls.us \
FORNAX_BENCH_DECODER=flashraw \
FORNAX_PROBE_DECODERS=flashraw \
FORNAX_PROBE_OUT=target/flashraw-031/probe.csv \
FORNAX_PROBE_WARMUPS=2 \
FORNAX_PROBE_ROUNDS=5 \
FORNAX_FLASHRAW_THREADS=1 \
RAYON_NUM_THREADS=1 \
cargo bench -p fornax-raw-decoder --bench flashraw_031_probe
```

The environment variables above are consumed by the checked-in
`flashraw_031_probe` benchmark. `FORNAX_FLASHRAW_BACKEND` optionally selects
`cpu`, `simd`, or `auto`; the default remains `cpu` for benchmark continuity.

## Decode-only improvement

The full per-sample table, including batch spread and minor faults, is
[flashraw-0.3.1-results.csv](data/flashraw-0.3.1-results.csv).

| Brand | Mode | Samples | Median decode delta | Best | Worst |
| --- | --- | ---: | ---: | ---: | ---: |
| DNG | uncompressed | 2 | -22.0% | -38.4% | -5.7% |
| DNG | lossless JPEG | 2 | +12.2% | +12.1% | +12.2% |
| Sony ARW | uncompressed | 2 | +3.8% | +2.8% | +4.8% |
| Sony ARW | lossless JPEG | 2 | -29.7% | -29.8% | -29.6% |
| Canon CR3 | lossless CRX | 2 | -3.7% | -4.8% | -2.6% |
| Nikon NEF | uncompressed words | 2 | +1.1% | -0.2% | +2.3% |
| Nikon NEF | lossless Huffman | 2 | -27.6% | -28.5% | -26.8% |

The lossless Sony and Nikon gains exceed the observed decode batch spread.
The DNG lossless regression also exceeds batch spread and should not be
treated as noise.

## End-to-end improvement

End-to-end is decode -> active-area crop/cast to `f32` -> preprocess.

| Brand | Mode | Samples | Median delta | Best | Worst |
| --- | --- | ---: | ---: | ---: | ---: |
| DNG | uncompressed | 2 | -13.6% | -23.0% | -4.3% |
| DNG | lossless JPEG | 2 | +7.2% | +5.5% | +8.9% |
| Sony ARW | uncompressed | 2 | +4.9% | -12.0% | +21.9% |
| Sony ARW | lossless JPEG | 2 | -19.7% | -23.0% | -16.5% |
| Canon CR3 | lossless CRX | 2 | +1.7% | +0.2% | +3.1% |
| Nikon NEF | uncompressed words | 2 | +0.7% | -2.6% | +4.0% |
| Nikon NEF | lossless Huffman | 2 | -12.7% | -12.7% | -12.7% |

The Sony uncompressed end-to-end result is inconclusive because its batch
spread overlaps the delta. Other uncompressed and CR3 changes are small
relative to host variance. The compressed Sony and Nikon improvements survive
end-to-end because decode remains a material share of the pipeline.

## Uncompressed versus lossless

At one thread, 0.3.1 delivered approximately the following raw-output
throughput. This uses two bytes per output sample and therefore represents
decoder output bandwidth, not compressed input bandwidth.

| Format | Compression | Output throughput |
| --- | --- | ---: |
| DNG | uncompressed | 2.3-2.5 GB/s |
| DNG | lossless JPEG | 0.42-0.47 GB/s |
| Sony ARW | uncompressed | 2.89-2.96 GB/s |
| Sony ARW | lossless JPEG | 0.41-0.43 GB/s |
| Canon CR3 | lossless CRX | 0.23-0.27 GB/s |
| Nikon NEF | uncompressed words | 2.76-2.78 GB/s |
| Nikon NEF | lossless Huffman | 0.56-0.62 GB/s |

Uncompressed decode rates cluster near 2.3-3.0 GB/s, consistent with a
memory/copy-bound path. Lossless rates are 0.23-0.62 GB/s, with CRX slowest and
Nikon Huffman fastest. This ordering is plausible; no case shows an
uncompressed decoder burdened like a compressed codec.

The full pipeline does not retain that ordering. On uncompressed files,
decode is only 14-18% of pipeline time for the large DNG/ARW/NEF samples; the
glue crop/cast is the majority. The combined pipeline time is therefore not a
good proxy for FlashRaw codec efficiency.

## Stage breakdown and thread scaling

The checked-in stage data is
[flashraw-0.3.1-stage-breakdown.csv](data/flashraw-0.3.1-stage-breakdown.csv).

For FlashRaw 0.3.1 at one thread, representative pipeline shares were:

| Sample | Decode | Crop/cast | Preprocess |
| --- | ---: | ---: | ---: |
| Pentax K-1 lossless DNG | 54.8% | 63.4% | 4.2% |
| Pixel 8 Pro lossless DNG | 53.2% | 66.6% | 4.7% |
| Leica SL2 uncompressed DNG | 17.7% | 83.3% | 7.8% |
| Sony ILCE-7RM5 lossless ARW | 53.4% | 62.7% | 4.2% |
| Sony ILCE-1 uncompressed ARW | 14.6% | 64.8% | 8.1% |
| Canon EOS R5 Mark II CR3 | 66.4% | 53.2% | 2.7% |
| Nikon Z7 lossless NEF | 49.3% | 63.9% | 5.4% |
| Nikon D850 uncompressed NEF | 16.3% | 61.8% | 8.7% |

Shares can sum above 100% because each stage is sampled independently and
cache/allocator state differs in the combined path.

Moving from one to sixteen threads:

- Lossless decode improved 2.3-5.3x on ARW, 1.9-5.1x on DNG, 3.2x on large
  CR3, and 2.9-3.1x on NEF.
- The FlashRaw adapter's scalar crop/cast mostly remained near one-thread
  speed; apparent gains of 1.4-1.6x on a few large samples are allocator and
  memory-state variation, not parallel scaling in the adapter.
- Preprocessing did not speed up because the measured large-image stage is
  already memory-bandwidth-bound at about 20 GB/s including read and write.
- End-to-end 3.1 speedups were about 1.3-1.9x for compressed files and near
  1.0x for most uncompressed files.

Minor-fault accounting agrees with the allocation path. A 50 MP `f32`
full-frame temporary has about 190 MiB of untouched pages. Measured large
crop/cast stages created roughly 35,000-63,000 minor faults and then
`shrink_to_fit` reallocated the retained crop. Preprocess added no new pages
because it normalized the existing cropped allocation in place.

## Profiling

`perf stat -r 3` on the Pentax K-1 case measured 1.87 IPC, 1.01% branch
misses, and 3.73% cache-reference misses across the complete probe workload,
including hashing and repeated stage sampling.

`perf record` attributed the dominant Pentax LJPEG cycles to
`flashraw_codecs::ljpeg::decode_row` (61.6% in the aggregate workload).
For the Leica uncompressed DNG, the largest symbols were the packed-row
decoder, the preprocessor loop, and the scalar crop/cast path. The profile
included probe hashing, so only the ordering and the clear LJPEG/crop split
should be carried forward.

## Fastest available FlashRaw options

The adapter had been forcing `Backend::Cpu`. FlashRaw defines that as the
portable scalar reference backend; `Backend::CpuSimd` selects runtime-dispatched
SIMD kernels, and `Backend::Auto` selects the fastest available backend. The
default features compiled here include `simd` and do not include `gpu`.

One-thread `CpuSimd` produced bit-identical raw and cropped outputs. Relative
to scalar:

- Decode improved 2-4% on most large samples.
- The small Sigma fp uncompressed DNG improved about 40%.
- End-to-end improved 1-4% on most samples and about 24% on the small DNG.

`DecodeOptions::threads` is already configurable and was honored correctly.
FlashRaw also exposes `decode_region`, but the adapter currently decodes the
full frame and performs active-area conversion afterward. No output-buffer
reuse API is used.

## Ranked opportunities

1. **Crop/cast directly into the active-area destination, then fuse
   normalization when practical.** This is Fornax glue work. It removes a
   full-frame conversion, a second allocation, page faults, and a redundant
   pass. Expected impact: 20-60% end-to-end on large files, with the largest
   gains on uncompressed inputs. Effort: medium. Recommended first.
2. **Use `Backend::Auto` or `CpuSimd` instead of scalar `Cpu`.** Expected
   impact: 1-4% end-to-end on large files and about 24% on the small
   uncompressed DNG; all outputs remained bit-identical. Effort: low.
3. **Use FlashRaw `decode_region` for the active area where the format can
   avoid decoding outside it.** Expected impact: additive with direct
   crop/cast for tiled and row-addressable formats; single entropy streams
   still need full decode. Effort: medium.
4. **Report the DNG lossless JPEG regression upstream.** The 0.3.0 rewrite
   changed the predictor-1 fast loop while preserving output. Recovering the
   prior path should recover about 12% decode and 5-9% end-to-end on these
   DNG samples. Effort: medium and upstream-dependent.
5. **Seek or design output-buffer reuse.** The current API returns a new
   owned frame. Reusing a caller-owned normalized output could remove another
   allocation and leave only one write pass. Expected impact: moderate on
   large images, but depends on an upstream API. Effort: high.
6. **Optimize preprocessing only after fusion.** Standalone preprocessing is
   already near memory bandwidth and is only 1-9% of the measured pipeline.
   Expected standalone impact is low. Effort: medium.

Recommended sequence: implement opportunity 1 with opportunity 2, verify exact
outputs, then evaluate `decode_region` as part of the same helper boundary.
File the DNG LJPEG regression upstream in parallel.
