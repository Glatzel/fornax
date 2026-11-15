# Portable LJPEG instruction and latency study

Host-specific evidence for PR #734 successors. Host: WSL2 Debian, AMD Ryzen 7
5800H, 15 visible CPUs, kernel `6.6.87.1-microsoft-standard-WSL2`, glibc
`2.41-12+deb13u3`, THP `[madvise]`, THP defrag `[madvise]`. Do not compare
numbers across host sessions; ratios below use same-session FlashRaw.

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
