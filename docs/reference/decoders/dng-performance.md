# DNG Decode Performance Reference Notes

This note records the 2026-11-14 investigation into Fornax DNG decode
performance relative to Rawler 0.8.0 and FlashRaw 0.2.0. It preserves the
measurement method, fairness constraints, hot paths, and implementation
differences that inform [the DNG performance plan](../../plans/perf-dng-decode.md).

Timing values in this note are host-specific observations. They are not a CI
baseline and must not be treated as portable performance guarantees.

## Sources

- Rawler 0.8.0, package source and JPEG/DNG decoder implementations.
  License: LGPL-2.1. Rawler remains a development-only behavioral reference.
- FlashRaw 0.2.0, package source and codec implementations.
  License: MIT OR Apache-2.0. See [FlashRaw reference notes](flashraw.md).
- The checked-in decoder benchmark:
  [crates/fornax-raw-decoder/benches/decoder.rs](../../../crates/fornax-raw-decoder/benches/decoder.rs).
- The pinned raw.pixls.us corpus and golden manifests under
  `crates/decoder-tracker/golden/`.
- Adobe DNG 1.7.1.0 and ITU-T T.81 for lossless JPEG behavior. See
  [DNG and lossless JPEG-92 reference notes](dng-lossless-jpeg.md).

## Environment and method

- WSL2 Debian on an AMD Ryzen 7 5800H, 15 logical CPUs, 9.7 GiB RAM.
- Rust/Cargo 1.100.0, x86-64 Linux.
- Release/bench defaults: optimized, `lto = false`, `codegen-units = 16`, no
  debug symbols.
- `RAYON_NUM_THREADS=1`, `OMP_NUM_THREADS=1`, and FlashRaw `threads = Some(1)`.
- Files were page-cache hot. Cold-storage latency was not measured.

The checked-in benchmark was used for adapters, input selection, and preflight
behavior. Locally, its CodSpeed-Criterion compatibility build entered
validation mode instead of printing wall times. The investigation therefore
used a temporary wall-clock probe through the same adapters and removed it
afterward.

Each headline case used three processes with 20 interleaved measurements per
decoder. Means and pooled standard deviations below are over 60 samples.

## Fairness scope

The checked-in benchmark's original method names are not directly comparable
because:

- Fornax `decode` stops after file read and TIFF parsing.
- Fornax pixel work occurs in `raw_mosaic`.
- Rawler and FlashRaw `decode` include pixel decoding.
- Reference `raw_mosaic` measurements clone the already decoded sample buffer.

The fair headline used in the investigation was:

```text
Fornax Decoder::decode(...).raw_mosaic()
vs Rawler rawler::decode_file(...)
vs FlashRaw decode(...)
```

All measurements include file opening, metadata parsing, pixel decode, and
native output allocation. They exclude crop and sample-type conversion.

Output storage matched at U16 or F32 as appropriate. Rawler and FlashRaw
normalize metadata into their own models, while Fornax retains and clones a
typed TIFF `Directory`; that difference is part of Fornax's measured cost.

The selected DNGs contain no linearization table or `OpcodeList1`. Pixel 8
contains `OpcodeList2`, which Fornax ignores and FlashRaw explicitly disables
with `apply_opcodes = false`. A cross-decoder pixel-hash gate did not yet exist;
the plan adds one before optimization.

## Timing results

Times are milliseconds, mean plus or minus pooled standard deviation.

| File | Variant | Fornax | Rawler | FlashRaw | Fornax / Rawler |
| --- | --- | ---: | ---: | ---: | ---: |
| 5D3 uncompressed | U16, one strip | 145.4 ± 8.6 | 48.7 ± 1.6 | 19.6 ± 4.2 | 2.98x |
| Sigma fp 4K | 12-bit, one strip | 66.3 ± 4.5 | 11.3 ± 4.4 | 10.5 ± 0.4 | 5.86x |
| Sigma fp 2K | 12-bit, one strip | 4.83 ± 0.49 | 1.38 ± 0.20 | 2.59 ± 0.14 | 3.50x |
| 5D3 lossless | U16 LJPEG, 384 256x256 tiles | 459.2 ± 16.0 | 262.3 ± 4.7 | 101.6 ± 7.7 | 1.75x |
| Pentax K-5 | 14-bit LJPEG, one strip | 358.3 ± 24.4 | 155.1 ± 16.5 | 88.5 ± 3.9 | 2.31x |
| iPhone SE | U16 LJPEG, 64 tiles | 185.3 ± 1.7 | 79.6 ± 0.7 | 43.3 ± 0.7 | 2.33x |
| Pixel 8 Pro | U16 LJPEG, 768 256x256 tiles | 1189.6 ± 67.9 | 565.5 ± 44.2 | 200.3 ± 8.4 | 2.10x |
| 5D3 HDR | FP16 predictor Deflate | 324.7 ± 34.8 | 449.6 ± 40.5 | 205.2 ± 15.3 | 0.72x |
| 5D3 HDR | FP32 predictor Deflate | 646.3 ± 35.0 | 802.3 ± 41.4 | 338.4 ± 13.4 | 0.81x |

Run-to-run means for the large files varied by roughly 0.5–4%. The small
Sigma 2K case had higher relative variance.

## Stage attribution

A separate temporary probe timed Fornax's parser-only path and pixel accessor
against FlashRaw `open_path` and `decode`:

| Case | Fornax parser/setup | Fornax pixels | FlashRaw open | FlashRaw decode |
| --- | ---: | ---: | ---: | ---: |
| 5D3 uncompressed | 115.4 ms | 43.0 ms | 0.25 ms | 14.9 ms |
| 5D3 LJPEG | 8.6 ms | 408.9 ms | 0.28 ms | 95.6 ms |
| FP32 Deflate | 130.6 ms | 547.6 ms | 0.10 ms | 325.7 ms |
| Pixel 8 LJPEG | 151.7 ms | 990.9 ms | 2.07 ms | 183.0 ms |

FlashRaw's `open_path` maps the file lazily; its decode time includes page
faults as sensor data is touched. Fornax reads into a `Vec<u8>` and TIFF parsing
constructs another `Arc<[u8]>`, so uncompressed setup pays a full-file copy
before decoding.

Pixel 8's `dng_metadata()` clone alone measured about 102.7 ms. Its merged
metadata expands to roughly 50 MB, including a private tag represented as more
than three million individual `Value` entries.

## Profile hot paths

Profiles used release optimization with DWARF symbols and `perf record
--call-graph dwarf`.

### 5D3 16-bit LJPEG

Fornax:

- `fornax_codec::ljpeg::decode_scan_into`: 57.3%
- `fornax_codec::ljpeg::add_prediction`: 32.3%
- `fornax_codec::ljpeg::BitReader::peek`: 5.6%
- TIFF parsing and source copying: below 1% of samples

FlashRaw:

- fused `decode_row_pred1`: 88.2%
- scan byte unstuffing: 3.2%
- Huffman table construction: 3.2%

For 20 decodes, the counters were:

| Decoder | Instructions | Branch misses | Sys time |
| --- | ---: | ---: | ---: |
| Fornax | 108.0 B | 269.9 M, 1.65% | 1.83 s |
| Rawler | 55.1 B | 62.5 M, 0.60% | 1.83 s |
| FlashRaw | 20.9 B | 13.3 M, 0.53% | 0.26 s |

Fornax executed about 5.2x FlashRaw's instructions and incurred about 20x its
branch misses on this case.

### Pixel 8 LJPEG

- `decode_scan_into`: 53.8%
- `add_prediction`: 30.9%
- `BitReader::peek`: 5.2%
- metadata cloning was a separate roughly 9% stage cost

### FP32 predictor Deflate

Fornax:

- miniz inflate: 48.8%
- `predictor::reverse`: 25.3%
- Deflate match application: 3.4%
- Huffman tree initialization: 3.3%
- float materialization: 2.0%

FlashRaw:

- zlib-rs inflate: 45.6%
- fused `decode_float_row`: 28.6%
- Adler-32: 8.1%

The profile shares describe where each decoder spends its own time; they do not
directly quantify the gap between decoders.

## Implementation differences

### LJPEG

Fornax:

- Uses a 16-bit symbol-only Huffman lookup.
- Performs a second bit peek for difference/amplitude bits.
- Uses a right-aligned cache, checks four bytes for `0xff`, and masks the cache
  on each skip.
- Executes a generic component loop and dynamic predictor dispatch for every
  sample.
- Allocates a full tile result, copies it into a band scratch buffer, then
  copies rows into the final mosaic.

Rawler:

- Uses a 13-bit decode cache that can return a completed difference directly.
- Writes tiles directly into the final image buffer.
- Uses Rayon tile parallelism in the benchmarked one-worker configuration.

FlashRaw:

- Uses a 12-bit full-difference LUT with a canonical slow path.
- Uses a left-aligned 64-bit cache and an 8-byte refill.
- Fuses predictor 1 for one or two components with direct row output.
- Keeps the general predictor path as fallback.

### Uncompressed

Fornax:

- Uses `u16::from_le_bytes` per sample even for native-endian U16.
- Unpacks 12/14-bit samples into `Vec<u32>` scratch, then makes a second pass
  into U16.

FlashRaw's benchmark backend selects CPU/scalar mode, not its SIMD backend. It
still uses a typed memcpy for native-endian U16 and grouped word kernels for
packed 12/14-bit samples.

### Deflate

Fornax:

- Inflates each tile into a newly allocated `Vec<u8>`.
- Runs predictor reversal as a whole-tile pass.
- Runs typed sample conversion as another pass.
- Has no reusable inflate scratch in the DNG path.

FlashRaw:

- Reuses per-thread scratch.
- Fuses predictor reversal, byte-plane unshuffle, and float conversion directly
  into the destination rows.

### TIFF and metadata

Fornax:

- `fs::read` copies the file into `Vec<u8>`.
- TIFF parsing copies the complete source into `Arc<[u8]>`.
- Parsing stores all directories and clones the root directory.
- `raw_mosaic` merges root and raw metadata by cloning both maps.
- TIFF BYTE/UNDEFINED payloads are expanded to one `Value` enum per byte.

Rawler and FlashRaw map source files. Their metadata models retain or parse
private payloads without building Fornax's full typed `BTreeMap`.

### Non-copying source storage

Task 3 makes the DNG decoder boundary map files read-only by default, passes
`TiffSource` into `fornax-tiff`, and returns the root directory by reference
instead of cloning directory zero. Disabling the `mmap` feature keeps an owned
`Arc<Vec<u8>>` fallback sourced directly from `fs::read`.

Single-thread walltime measurements from the Task 3 base (`740446c`) to the
implementation:

| Case | Base | Implementation |
| --- | ---: | ---: |
| 5D3 uncompressed end-to-end | 151.2 ms | 51.6 ms |
| 5D3 FP32 Deflate end-to-end | 761.6 ms | 528.7 ms |
| 5D3 FP32 Deflate parse-only | 138.1 ms | 0.047 ms |

The uncompressed Fornax/Rawler ratio fell from 2.98x to 1.01x.

The mapping is read-only but does not protect against another process truncating
or replacing the file contents while a decoder is alive. Such mutation can
raise `SIGBUS` on access. Consumers that require protection from this behavior
must disable the default `mmap` feature and use the owned-buffer fallback.

## Neutral experiments

- `-C target-cpu=native` measured 475 ms LJPEG, 164 ms uncompressed, and
  620 ms FP32 versus 459/145/646 ms for generic x86-64. It was neutral or
  worse.
- Switching Fornax Deflate from miniz to zlib-rs improved FP32 from 646.3 ms
  to 632.7 ms, about 2%.
- FlashRaw's benchmark adapter requests `Backend::Cpu`, which disables its
  runtime SIMD selection. SIMD is not required to explain the benchmark gap.
- The fair benchmark fixes all decoders to one worker, so a multi-thread versus
  single-thread comparison is not part of these numbers.

## Blackmagic 12-bit packed-row finding

The materialized Blackmagic Pocket Cinema Camera 4K lossless DNG was originally
rejected as:

```text
lossless JPEG frame is 2064x1088, expected 1032x2176
```

Its first tile begins with a valid SOF3 frame:

- precision 12;
- one component;
- JPEG frame 2064x1088;
- TIFF tile 1032x2176;
- identical total sample count.

This is a lossless packed-row layout: two TIFF rows are stored per JPEG row.
The current tile assembler requires exact row geometry and needs a linear
reshape path. This finding is independent of the performance work.

## Coverage gaps

- No integer predictor-Deflate corpus case was benchmarked.
- No cold-storage comparison was made.
- Multi-thread scaling was not measured because the fair baseline is
  single-threaded.
- Cross-decoder byte equality was not yet a checked-in gate during the
  investigation.
- The checked-in Criterion target's Fornax parse-only versus reference
  end-to-end mismatch still needs benchmark-structure correction.
