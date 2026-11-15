# Portable LJPEG instruction and latency study

Host-specific evidence for PR #734 successors. Host: WSL2 Debian, AMD Ryzen 7
5800H, 15 visible CPUs, kernel `6.6.87.1-microsoft-standard-WSL2`, glibc
`2.41-12+deb13u3`, THP `[madvise]`, THP defrag `[madvise]`. Do not compare
numbers across host sessions; ratios below use same-session FlashRaw.

## Linux large-page allocation

The current codec restores the first-touch policy as one platform-specific
module, [`large_alloc.rs`](../../../../crates/fornax-codec/src/large_alloc.rs).
Linux with the default-on `large-page-hint` feature uses natural-alignment
`alloc_zeroed` for final outputs at least 8 MiB, advises the 2 MiB-aligned live
interior with `MADV_HUGEPAGE`, and adopts the block into the owned `Vec`.
Windows, macOS, Miri, feature off, and smaller outputs use a plain zeroed `Vec`.
The public decode API is unchanged.

All platform code and unsafe operations are in that module. It is called from
`samples.rs` (integer, float, uncompressed, and the final image stage of the
other payload codecs), `deflate.rs`, `ljpeg.rs`, `jpeg.rs`, and `jpegxl.rs`.
LJPEG's <=8-bit final U8 conversion is routed through it as well. The module has
two production unsafe blocks: allocation plus Vec adoption, and `madvise`.
Counting-allocator tests and their unsafe trait implementation stay in the same
file.

The exact same-session regression table against PR #736 is:

| File | Threads | Plain `Vec` | With module | Change | With/FlashRaw |
| --- | ---: | ---: | ---: | ---: | ---: |
| 5D3 uncompressed | 1 | 47.357 ms | 18.400 ms | -61.1% | 1.008x |
| 5D3 uncompressed | 15 | 47.036 ms | 18.070 ms | -61.6% | 0.982x |
| Sigma fp | 1 | 5.794 ms | 5.635 ms | -2.7% | 0.507x |
| Sigma fp | 15 | 5.850 ms | 5.742 ms | -1.8% | 0.502x |
| 5D3 LJPEG | 1 | 177.101 ms | 143.696 ms | -18.9% | 1.514x |
| 5D3 LJPEG | 15 | 31.790 ms | 30.092 ms | -5.3% | 0.321x |
| Pixel 8 LJPEG | 1 | 363.639 ms | 288.105 ms | -20.8% | 1.508x |
| Pixel 8 LJPEG | 15 | 61.866 ms | 54.165 ms | -12.4% | 0.284x |
| iPhone SE LJPEG | 1 | 61.711 ms | 61.455 ms | -0.4% | 1.493x |
| iPhone SE LJPEG | 15 | 13.836 ms | 13.307 ms | -3.8% | 0.321x |
| 5D3 FP16 Deflate | 1 | 239.522 ms | 180.030 ms | -24.8% | 0.949x |
| 5D3 FP16 Deflate | 15 | 54.220 ms | 44.281 ms | -18.3% | 0.233x |
| 5D3 FP32 Deflate | 1 | 401.427 ms | 340.140 ms | -15.3% | 1.035x |
| 5D3 FP32 Deflate | 15 | 66.707 ms | 60.219 ms | -9.7% | 0.180x |

Medians use three fresh interleaved processes x five rounds. The 1T and 15T
FP32 ranges are disjoint, and no measured case regresses. The suggested 20%
relative gate is not met by FP32, but the 1T result reproduces the earlier
approximately 60 ms absolute win; the keep decision is recorded in
[the main gap report](../dng-remaining-gap.md).

On 5D3, fresh-process one-thread `AnonHugePages` is 43,008 KiB for uncompressed
and LJPEG and 88,064 KiB for FP32. Minor faults change from 12,750 to 2,021,
40,091 to 29,360, and 24,316 to 2,342 respectively. The host blocks setting THP
to `never`, so a feature-off binary is the fallback proxy; it stays within
process noise. Full ranges, fallback values, cross-target results, and evidence
limits are in
[the large-page allocation evidence](large-page-allocation.txt). These numbers
are Linux-only and unmeasured on Windows or macOS.

When maintaining the module, re-check the `Layout::array::<T>(len)` natural
alignment, the same-global-allocator Vec adoption with `capacity == len`, the
8 MiB threshold, and the `AnonHugePages`/minor-fault probe after Rust, glibc, or
kernel changes. Over-alignment is a regression: it makes the allocation
zero-fill before `madvise` and silently disables the first-touch benefit.

Follow-up LJPEG compute work starts with the
[FlashRaw 0.2.0 CPU source comparison](../flashraw-cpu-comparison.md); it is a
design note, not an accepted implementation change.

## Reproduction

Build the exact-input probe:

```bash
CARGO_PROFILE_RELEASE_DEBUG=line-tables-only \
  cargo build -p fornax-raw-decoder --release --bench dng_gap_probe
```

Exact instruction attribution:

```bash
RAYON_NUM_THREADS=1 valgrind --tool=callgrind --dump-instr=yes \
  --collect-jumps=yes --callgrind-out-file=/tmp/ljpeg.callgrind \
  target/release/build/fornax-raw-decoder/*/out/dng_gap_probe-* \
  decode --decoder fornax --threads 1 --warmups 0 --rounds 1 \
  external/raw.pixls.us/Adobe\ DNG\ Converter/Canon\ EOS\ 5D\ Mark\ III/5G4A9394-compressed-lossless.DNG
```

Native counters and wall time:

```bash
RAYON_NUM_THREADS=1 perf stat -x, \
  -e instructions:u,cycles:u,branches:u,branch-misses:u,L1-dcache-load-misses:u,stalled-cycles-frontend:u \
  -- target/release/build/fornax-raw-decoder/*/out/dng_gap_probe-* \
  decode --decoder fornax --threads 1 --warmups 0 --rounds 5 INPUT
```

Use `RAYON_NUM_THREADS=15 --threads all` for all-thread runs. Strict equality:

```bash
RAYON_NUM_THREADS=1 target/release/build/fornax-raw-decoder/*/out/dng_gap_probe-* \
  equality INPUT
```

Linked worktrees must reuse the main checkout's populated `external/` data with
hard links (`rsync --link-dest`, excluding `.git`). Treat those files as
read-only because all hard links share the inode.

## Final wall, cycles and FlashRaw

Three fresh interleaved processes, five rounds each. Wall is median over 15
rounds. Counters use the 5D3 cropped output denominator of 23,384,000 samples.

| File | Threads | Wall before | Wall after | After/before | Cycles/sample before | After | IPC before | After | FlashRaw cycles | After/FlashRaw wall |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 5D3 | 1 | 228.754 ms | 201.121 ms | 0.879x | 29.055 | 24.361 | 3.463 | 2.353 | 15.139 | 1.936x |
| 5D3 | 15 | 45.318 ms | 38.228 ms | 0.844x | 47.929 | 33.451 | 2.102 | 1.717 | 15.153 | 0.361x |
| Pixel 8 | 1 | 444.913 ms | 383.196 ms | 0.861x | 27.636 | 23.064 | 3.348 | 2.300 | 14.345 | 1.864x |
| Pixel 8 | 15 | 83.364 ms | 70.369 ms | 0.844x | 43.872 | 30.886 | 2.110 | 1.719 | 14.176 | 0.353x |
| iPhone SE | 1 | 78.403 ms | 64.400 ms | 0.821x | 24.088 | 20.135 | 3.721 | 2.511 | 12.965 | 1.444x |
| iPhone SE | 15 | 20.950 ms | 14.731 ms | 0.703x | 31.749 | 22.859 | 2.827 | 2.218 | 12.790 | 0.339x |

The 15T FlashRaw wall values were anomalously slower than its 1T values.
Instruction and cycle ratios are stable. Wall improves 12.1-29.7% while exact
instructions fall 39.48%; IPC falls because the serial dependency floor becomes
dominant.

## Exact instruction progression

5D3 one-thread Callgrind:

| Revision | Instructions | Per decoded LJPEG sample |
| --- | ---: | ---: |
| PR #734 | 2,564,082,227 | 101.887 |
| truncation helpers | 2,361,790,125 | 93.849 |
| component-local predictors | 2,152,706,981 | 85.541 |
| fixed table | 2,038,261,237 | 80.994 |
| word refill | 1,651,521,827 | 65.625 |
| packed entries | 1,551,617,888 | 61.659 |

The padded entropy domain is 25,165,824 samples. The cropped 5D3 output is
23,384,000 samples, which yields the historical ~100.7 baseline
instructions/sample.

## Kept and reverted decisions

Kept:

- Truncation helpers: -202,292,102 instructions; real instructions/sample
  improve about 8.6-8.9% and wall medians improve.
- Component-local predictors and chunked row writes: -209,083,144 instructions;
  native instructions/sample improve about 9-10%. 15T wall is flat within spread.
- Fixed-size `8192` fast table: -114,445,744 instructions; native
  instructions/sample improve about 5.9%. Pixel 8 15T wall regresses within
  spread.
- Word refill: -386,739,410 instructions; branch misses roughly halve and all
  five earlier real decoders improve in wall time.
- Packed `u32` fast entry: -99,903,939 instructions; four wall medians improve,
  iPhone SE wall is flat within spread.

Reverted:

- Peeled row/edge loops: +1,099,021 instructions and every real-file
  instructions/sample regressed despite helping synthetic branch-heavy scans.

## Final exact hot-loop costs

Callgrind self-cost highlights within the final 1,551,617,888-instruction run:

| Site | Instructions | Per decoded sample |
| --- | ---: | ---: |
| fast-table entry load | 75,497,472 | 3.000 |
| packed `full` extraction/branch | 25,165,824 | 1.000 |
| predictor select | 75,497,472 | 3.000 |
| fast Huffman call/site | 75,497,472 | 3.000 |
| refill buffered-bits branch | 75,497,472 | 3.000 |
| refill range check | 16,713,052 | 0.664 |
| refill cache append/taken calculation | 36,398,481 | 1.446 |
| peek truncation/shift | 77,680,620 | 3.087 |
| cache/buffer skip | 26,621,256 | 1.058 |
| output store | 25,165,824 | 1.000 |
| predictor update | 25,165,824 | 1.000 |
| point-transform branch | 25,165,824 | 1.000 |
| slow 16-bit Huffman path | 2,546,964 | 0.101 |

Top-level exact groups: 878,328,164 instructions in the inlined row closure,
138,608,640 in slice/chunk operations, 125,704,301 + 107,957,446 in libc
copy/fill/calloc leaves, and 61,289,728 + 49,969,710 in numeric conversion
helpers.

## Lookup block and dependency chain

Final fast-hit assembly:

```text
mov    %r13,%rdi                 cache -> index scratch
shr    $0x33,%rdi                top 13 bits
mov    0x10(%rcx),%...           table base
mov    (%base,%index,4),%eax     packed entry load
test   $0x20,%al                 full flag
jne    full_hit
...
full_hit:
mov    %eax,%ecx                 packed entry -> shift scratch
and    $0x1f,%ecx                consumed bits
shl    %cl,%rbx                  next cache
mov    %rbx,0x48(%rsp)           cache spill
```

Fornax loop-carried chain: cache index shift, packed-entry load, extracted
consumed shift count, variable cache shift, cache stack store, next-iteration
reload. Estimated latency: 1 shift + 4-5 load-to-use + 1-2 extraction/moves +
1 shift + 4-5 store-forwarding = **11-14 cycles/sample**. Other work can overlap,
but the next table lookup cannot start until the shifted cache is available.

FlashRaw keeps the equivalent bit-pump cache and count in registers and uses a
direct table symbol/shift path. Its loop has the same unavoidable table-load
and variable-shift dependency, but no Fornax-style cache spill/reload. Estimated
chain is about **6-8 cycles/sample**.

Off the chain but still consuming cycles: predictor selection, remaining Vec
load/store bounds checks, packed-entry flag extraction, refill marker test,
row store, component loop control, precision mask load, and point-transform
branch. The refill branch and cache stack reload touch the chain; the row store
and loop control do not.

## Task B counters: 5D3 1T

Medians from three fresh processes x five rounds, same session:

| Metric per cropped sample | Fornax | FlashRaw |
| --- | ---: | ---: |
| Instructions | 57.329 | 40.696 |
| Cycles | 24.362 | 14.993 |
| IPC | 2.353 | 2.714 |
| Branches | 8.924 | 4.920 |
| Branch misses | 0.11699 | 0.02585 |
| L1D load misses | 0.96303 | 0.42870 |
| Stalled-cycles-frontend | 0.89820 | 0.22582 |

Backend stall counters are not exposed by this host.

Perf sampled branch-miss IPs cluster in the Fornax row closure and FlashRaw
`decode_row_pred1`; without precise branch sampling, per-branch attribution is
not reliable. Synthetic branch behavior is informative: smooth miss rate is
0.82% and noisy is 3.08%, showing that the rare full-entry miss and
branchy refill/amplitude paths become more expensive on noisy data. The
full-entry miss occurs on only 1.45% of 5D3 samples.

## Critical-path conclusion

The remaining removable chain cost is the cache stack spill/reload. Eliminating
it requires a broader reader-state refactor, while the safer candidates tested
during this branch are exhausted. Since the final branch is still 1.38-1.42x
FlashRaw in instructions and 1.44-1.94x in 1T wall time, and cycles remain
20-24 versus FlashRaw's 13-15, LJPEG is at the current scalar implementation's
latency floor on this host. Further instruction-only trimming is not expected to
close the gap; a reader-state/register-allocation experiment would be a separate
design change.

## Compatibility

- No package `rust-version` is declared. `first_chunk` is stable since Rust 1.77,
  so that is the effective floor for the new refill API.
- Big-endian Miri was attempted but unavailable: the stable toolchain has no
  Miri component. The `from_be_bytes` conversion is endian-explicit, but this
  portability check remains unrun.
- Windows and macOS target checks are covered by repository validation when
  their targets are installed.

## Critical-path follow-up after PR #736

This follow-up tested two bounded hypotheses from the merged PR #736 state:
move the bit-reader state out of an address-taken struct, and replace the
13-bit fused difference table with a compact branchless code table. All results
below are from the same WSL2 host named at the top of this file. They are
host-specific and must not be compared with another session.

The command shape was:

```bash
CARGO_PROFILE_RELEASE_DEBUG=line-tables-only \
  cargo build -p fornax-raw-decoder --release --bench dng_gap_probe
CARGO_PROFILE_RELEASE_DEBUG=line-tables-only \
  cargo build -p fornax-codec --release --bench ljpeg_gap_probe \
  --features lossless-jpeg

RAYON_NUM_THREADS=1 perf stat -x, \
  -e instructions:u,cycles:u,branches:u,branch-misses:u,\
L1-dcache-load-misses:u,stalled-cycles-frontend:u -- \
  target/release/build/fornax-raw-decoder/*/out/dng_gap_probe-* \
  decode --decoder fornax --threads 1 --warmups 0 --rounds 5 INPUT

perf stat -x, -e instructions:u,cycles:u,branches:u,branch-misses:u -- \
  target/release/build/fornax-codec/*/out/ljpeg_gap_probe-* \
  --components 1,2 --pattern smooth,noisy --tiles 256 --rounds 5
```

Real-file runs used three fresh processes, five interleaved rounds per process,
and a same-session FlashRaw process. Wall values are medians with min-max over
15 rounds. The 15T FlashRaw wall times on this WSL host remain anomalously
non-scaling, so 15T cycle ratios are more reliable than 15T wall ratios even
though both are reported as required.

### Exact baseline and final 5D3 1T counters

| Metric per cropped sample | Fornax baseline | Final 3a | FlashRaw |
| --- | ---: | ---: | ---: |
| Instructions | 57.329 | 79.581 | 40.696 |
| Cycles | 23.917 | 20.648 | 14.604 |
| IPC | 2.397 | 3.854 | 2.787 |
| Branches | 8.924 | 10.684 | 4.920 |
| Branch misses | 0.11672 | 0.10629 | 0.02583 |
| L1D load misses | 0.96167 | 0.06010 | 0.41886 |
| Frontend stalls | 0.85495 | 0.66386 | 0.22103 |

The final branchless table is faster despite retiring 39% more instructions:
the table working set and miss rate fall enough to raise IPC from 2.40 to 3.85.
The measured frontend-stall counter falls as well. Backend stalled-cycle
counters are not exposed by this host.

### Real-file before/after and FlashRaw ratios

`After/before` below one is faster. FlashRaw wall is the same-session value;
the final column is candidate wall divided by FlashRaw wall.

| Experiment | File | T | Wall before ms (min-max) | Wall after ms (min-max) | After/before | FlashRaw ms | After/FlashRaw | Cycles/sample before -> after | IPC before -> after | Instr/sample before -> after | Branch misses/sample before -> after |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| reader locals | 5D3 | 1 | 179.603 (177.456-269.290) | 184.238 (180.670-267.957) | 1.026x | 99.122 | 1.859x | 23.981 -> 24.809 | 2.391 -> 1.940 | 57.329 -> 48.126 | 0.11681 -> 0.11208 |
| reader locals | 5D3 | 15 | 34.786 (32.287-55.179) | 33.985 (31.611-47.608) | 0.977x | 96.949 | 0.351x | 32.037 -> 31.596 | 1.792 -> 1.526 | 57.426 -> 48.227 | 0.12063 -> 0.11592 |
| reader locals | Pixel 8 | 1 | 366.475 (360.670-551.704) | 367.012 (364.287-558.097) | 1.001x | 194.206 | 1.890x | 22.722 -> 23.117 | 2.335 -> 1.913 | 53.056 -> 44.214 | 0.11950 -> 0.11804 |
| reader locals | Pixel 8 | 15 | 64.358 (63.002-104.975) | 64.601 (62.039-104.582) | 1.004x | 191.395 | 0.338x | 29.329 -> 29.333 | 1.811 -> 1.509 | 53.109 -> 44.267 | 0.12371 -> 0.12233 |
| reader locals | iPhone SE | 1 | 62.619 (61.480-96.858) | 62.037 (60.599-96.412) | 0.991x | 41.478 | 1.496x | 19.942 -> 19.784 | 2.535 -> 2.080 | 50.557 -> 41.150 | 0.09333 -> 0.09431 |
| reader locals | iPhone SE | 15 | 14.629 (13.899-23.055) | 13.888 (12.646-23.048) | 0.949x | 40.901 | 0.340x | 22.566 -> 21.826 | 2.246 -> 1.892 | 50.692 -> 41.289 | 0.09622 -> 0.09706 |
| compact table | 5D3 | 1 | 175.302 (172.757-260.531) | 157.404 (155.480-159.081) | 0.898x | 94.130 | 1.672x | 23.917 -> 20.648 | 2.397 -> 3.854 | 57.329 -> 79.581 | 0.11672 -> 0.10629 |
| compact table | 5D3 | 15 | 33.446 (29.577-49.838) | 33.239 (30.486-35.747) | 0.994x | 95.550 | 0.348x | 31.664 -> 30.855 | 1.814 -> 2.582 | 57.426 -> 79.682 | 0.12068 -> 0.11025 |
| compact table | Pixel 8 | 1 | 360.765 (354.750-551.917) | 331.464 (319.813-341.789) | 0.919x | 196.844 | 1.684x | 22.752 -> 19.681 | 2.332 -> 3.753 | 53.056 -> 73.864 | 0.11947 -> 0.11101 |
| compact table | Pixel 8 | 15 | 61.651 (60.297-103.292) | 62.805 (60.562-78.167) | 1.019x | 199.775 | 0.314x | 29.111 -> 28.598 | 1.824 -> 2.585 | 53.109 -> 73.916 | 0.12355 -> 0.11571 |
| compact table | iPhone SE | 1 | 65.152 (61.817-102.525) | 65.126 (62.678-81.376) | 1.000x | 42.179 | 1.544x | 20.097 -> 19.543 | 2.516 -> 3.757 | 50.557 -> 73.430 | 0.09344 -> 0.09331 |
| compact table | iPhone SE | 15 | 15.010 (14.088-24.062) | 15.973 (11.208-21.690) | 1.064x | 42.721 | 0.374x | 23.020 -> 23.152 | 2.202 -> 3.177 | 50.691 -> 73.564 | 0.09634 -> 0.09527 |
| unconditional refill | 5D3 | 1 | 162.439 (160.795-165.972) | 178.382 (175.573-183.240) | 1.098x | 96.707 | 1.845x | 20.702 -> 23.330 | 3.844 -> 4.306 | 79.581 -> 100.463 | 0.10636 -> 0.01814 |
| unconditional refill | 5D3 | 15 | 32.173 (29.794-38.023) | 36.662 (34.331-42.447) | 1.140x | 96.009 | 0.382x | 30.913 -> 37.157 | 2.578 -> 2.706 | 79.684 -> 100.558 | 0.11013 -> 0.02003 |
| unconditional refill | Pixel 8 | 1 | 341.539 (330.611-357.396) | 362.564 (356.664-396.320) | 1.062x | 191.937 | 1.889x | 19.751 -> 21.965 | 3.740 -> 4.279 | 73.864 -> 93.984 | 0.11114 -> 0.01583 |
| unconditional refill | Pixel 8 | 15 | 64.785 (63.365-82.201) | 70.894 (70.142-88.302) | 1.094x | 196.197 | 0.361x | 28.136 -> 33.502 | 2.627 -> 2.807 | 73.916 -> 94.036 | 0.11540 -> 0.01705 |
| unconditional refill | iPhone SE | 1 | 63.476 (60.792-83.233) | 68.249 (67.336-88.377) | 1.075x | 42.016 | 1.624x | 19.473 -> 21.708 | 3.771 -> 4.270 | 73.430 -> 92.688 | 0.09322 -> 0.00510 |
| unconditional refill | iPhone SE | 15 | 15.839 (11.897-22.911) | 18.196 (17.594-24.662) | 1.149x | 41.672 | 0.437x | 23.154 -> 26.723 | 3.177 -> 3.474 | 73.567 -> 92.838 | 0.09544 -> 0.00635 |

### Synthetic smooth/noisy scans

Each value is the median of three fresh processes with five rounds over 256
256x256 tiles. Ratios are after/before; values below one are faster.

| Experiment | Components | Pattern | Before ms | After ms | Ratio |
| --- | ---: | --- | ---: | ---: | ---: |
| reader locals | 1 | smooth | 58.664 | 56.794 | 0.968x |
| reader locals | 1 | noisy | 150.109 | 139.092 | 0.927x |
| reader locals | 2 | smooth | 196.944 | 193.242 | 0.981x |
| reader locals | 2 | noisy | 292.276 | 302.932 | 1.037x |
| compact table | 1 | smooth | 58.532 | 71.875 | 1.228x |
| compact table | 1 | noisy | 149.532 | 76.529 | 0.512x |
| compact table | 2 | smooth | 197.817 | 138.768 | 0.702x |
| compact table | 2 | noisy | 290.788 | 145.429 | 0.500x |
| unconditional refill | 1 | smooth | 71.979 | 83.533 | 1.161x |
| unconditional refill | 1 | noisy | 75.166 | 85.903 | 1.143x |
| unconditional refill | 2 | smooth | 138.712 | 165.333 | 1.192x |
| unconditional refill | 2 | noisy | 146.440 | 167.998 | 1.147x |

The compact table is a trade rather than a universal synthetic win. It slows
the already branch-predictable one-component smooth case by paying the
amplitude and sign-extension arithmetic, but removes most table-miss and noisy
path cost. Real RAW tiles improved because their L1D behavior dominates that
arithmetic cost.

### Table footprint and cold paths

The retained table stores one two-byte `(length, symbol)` entry per 10-bit
prefix plus the canonical length and symbol arrays. For the measured 17-symbol
tables this is approximately 2.1 KiB per Huffman table instead of 32 KiB of
fused entries plus a 128 KiB 16-bit lookup. The largest supported four-table
tile therefore uses about 8.3 KiB of Huffman tables instead of about 640 KiB.

Temporary counters were removed after collection:

| Input | Samples | Fast entries | Cold entries | Cold fraction | Fast refills | Slow refills | Slow-refill fraction |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 5D3 | 25,165,824 | 25,164,800 | 1,024 | 0.00407% | 5,199,783 | 372,223 | 1.479% |
| Pixel 8 | 50,331,648 | 50,330,855 | 793 | 0.00158% | 9,535,461 | 506,264 | 1.006% |

PR #736's fused table had a 1.445842% miss rate on 5D3. The compact table
therefore reduces cold Huffman escapes by roughly 355x.

### Dependency chain and stack traffic

The PR #736 loop-carried chain was:

```text
cache load 0x158(%rsp)
-> cache >> 51 index shift
-> packed 4-byte table load
-> consumed extraction
-> variable cache shift
-> cache store 0x158(%rsp)
-> next-iteration reload
```

Estimated latency was 11-14 cycles/sample. The retained compact-table loop is:

```text
cache load 0x158(%rsp)
-> cache >> 54 index shift
-> 2-byte code-table load
-> length and symbol extraction
-> add length + symbol
-> one variable cache shift
-> cache store 0x158(%rsp)
-> next-iteration reload
```

Its estimated data-chain latency is 12-15 cycles/sample, so the chain is not
shorter. Amplitude extraction and branchless sign extension are off the
next-cache chain and overlap with it. The speedup comes from reducing L1D misses
from 0.96167 to 0.06010/sample and removing the frequently mispredicting fused
full-difference path, not from shortening the cache dependency itself.

FlashRaw's `decode_row_pred1` also stores and reloads bit-pump cache state at
`0x18(%rbx)` and has a comparable shift -> LUT load -> consumed-count ->
variable-shift chain. The earlier claim that only Fornax spills the cache is
not supported by the disassembly. FlashRaw's remaining advantage is fewer loop
control, component, and table-address instructions plus fewer branches.

The `0x48(%rsp)` address in the earlier report was from an older register
allocation. In the final disassembly cache, bit count, and position are at
`0x158`, `0x160`, and `0x150(%rsp)`. They remain in memory because the
address-taken `FastBitReader` is passed to the inlined table/refill code and to
cold helpers. Moving them to scalar locals removed the cache spill but
increased specialization and position traffic; Experiment 1 regressed real
1T cycles and was reverted.

### Branch behavior

Precise branch-stack attribution is unavailable: this host reports that the
branch-miss PMU event does not support branch stack sampling. Perf still shows
that 87.89% of branch-miss IP samples are in the Fornax row closure.
The smooth/noisy split isolates the behavior:

| Components | Pattern | Baseline branch misses/sample and rate | Compact-table branch misses/sample and rate | Baseline wall | Compact-table wall |
| ---: | --- | ---: | ---: | ---: | ---: |
| 1 | smooth | 0.05693, 0.650% | 0.00571, 0.047% | 63.269 ms | 85.904 ms |
| 1 | noisy | 0.54152, 3.254% | 0.01284, 0.104% | 150.947 ms | 85.372 ms |
| 2 | smooth | 0.05604, 0.731% | 0.00395, 0.036% | 189.956 ms | 166.210 ms |
| 2 | noisy | 0.54721, 3.521% | 0.01086, 0.096% | 371.766 ms | 166.630 ms |

The removed fused-table miss branch is the major noisy-data mispredict. The
temporary 5D3 counter measured 1.479% slow refills, and unconditional refill
reduced real branch misses/sample from 0.10636 to 0.01814. It still raised
cycles/sample from 20.702 to 23.330 because it added an eight-byte load and
more refill work to every sample. The refill branch is therefore not the
dominant cost, and no marker-unstuffing experiment was started.

### Decisions

| Experiment | Decision | Evidence |
| --- | --- | --- |
| reader locals | Reverted in `9a85782` | Cache spill disappeared and instructions/sample fell 57.329 -> 48.126, but 5D3 1T cycles regressed 23.981 -> 24.809 and 1T wall did not improve. |
| compact 10-bit branchless table | Kept in `f06cb2e` | 5D3 1T cycles 23.917 -> 20.648 and wall 175.302 -> 157.404 ms; Pixel 8 1T wall 360.765 -> 331.464 ms; exact three-way equality passed. |
| unconditional refill | Reverted in `0c35bce` | Branch misses/sample fell to 0.01814, but every real and synthetic case regressed in wall time and cycles. |

No further experiment was started. On this host the final portable scalar code
is 1.41x FlashRaw in 1T cycles and 1.67x in 1T 5D3 wall time. The remaining
serial cache chain and the branch/control cost around two-component rows are
still visible. Interleaving two independent tiles in one scalar loop is worth
proposing as a separate design experiment because it can expose a second
dependency chain to the core; it was outside this task's stop rule and was not
implemented.

### CodSpeed review regression

CodSpeed reported regressions for the Canon 60D compressed DNG and Sony
ILCE-9M3 lossless ARW while the Canon 5D3 DNG improved. The report also warned
that all six significant changes ran on different hardware, so the inputs were
re-measured locally against `985c704` with three fresh processes and five
rounds per process.

The 60D and Sony files use the general multi-component LJPEG route. The first
compact-table implementation restored success semantics there with a bitwise
canonical decoder and then a compact direct lookup. Assembly showed that the
new `HuffmanTable::decode` helper remained out of line, adding call and
register-shuffling overhead to every symbol:

```text
985c704: HuffmanTable::decode inlined into the scan loop
initial #737: out-of-line HuffmanTable::decode call per symbol
```

The fix forces the fast lookup inline and moves only long-code/error handling
to a cold noinline helper. This preserves the 2 KiB hot lookup rather than
restoring the 128 KiB table. Same-host medians before and after the review fix:

| Input | Baseline `985c704` | Initial #737 | Review fix | Initial/base | Fixed/base |
| --- | ---: | ---: | ---: | ---: | ---: |
| Canon 60D compressed DNG | 8.801 ms | 11.010 ms | 8.379 ms | 1.304x | 0.952x |
| Canon 5D3 compressed DNG | 12.080 ms | 10.823 ms | 10.823 ms | 0.896x | 0.896x |
| Sony ILCE-9M3 lossless ARW | 182.632 ms | 248.838 ms | 181.113 ms | 1.386x | 0.992x |

After the fix, 60D instructions/cycles are 1.051x/1.012x baseline while L1D
load misses are 0.247x. Sony is 1.016x/0.988x baseline with L1D misses at
0.306x. The 5D3 fast route remains at 0.869x cycles and 0.084x L1D misses.
