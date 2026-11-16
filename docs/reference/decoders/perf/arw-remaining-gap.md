# Sony ARW Decode Remaining Performance Gap

This report compares the portable-scalar Fornax decoder with Rawler 0.8.0 and
FlashRaw 0.2.0 for Sony ARW. It covers the two requested codec families:
uncompressed CFA strips and real lossless JPEG. Lossy ARW2, delta ARW6, and
unverified layouts were not decoded.

## Result

Fornax is 8.22x slower than FlashRaw on the trusted 14-bit uncompressed case
and 2.31x to 4.79x slower on the trusted lossless-JPEG cases. Rawler is faster
than Fornax in every trusted measurement except 14-bit LJPEG at 15 threads,
where Fornax is 0.99x Rawler.

Unlike DNG, uncompressed ARW is not primarily an output-first-touch story.
The dominant extra cost is the ARW container boundary: Fornax copies the
source once for `fs::read` and the TIFF parser copies it again while parsing.
For lossless JPEG, the image data is real ISO/IEC 10918 lossless JPEG
(`SOI`, `SOF3`, four components), but it takes the generic interleaved Sony
path rather than DNG's predictor-1 fast row path. Fornax executes about
187-199 instructions per output sample versus FlashRaw's 63-64.

The 12-bit uncompressed gate failed on every sampled body, so no 12-bit
uncompressed ratio is reported as trusted. This is a correctness finding rather
than a performance result.

## Method

- Release build from PR #734 state (`65cbb38`).
- Page-cache-hot inputs.
- Fornax:
  `Decoder::decode(path).raw_mosaic()`.
- Rawler: `decode_file`.
- FlashRaw: `Backend::Cpu`, strict corruption handling,
  `apply_opcodes = false`, `crop = false`, and 1 or 15 workers.
- Three fresh processes per case and thread count.
- Five interleaved rounds per process after one warmup, for 15 observations per
  decoder.
- Rawler has no explicit worker argument in the benchmark adapter; its 15T
  values are its API behavior under the shared environment.
- Ratios were accepted only after exact dimensions, U16 storage, source sample
  bits, and every pixel matched across all three decoders at both thread
  counts.

Host: WSL2 Debian, AMD Ryzen 7 5800H, 8 physical cores and 15 visible SMT CPUs.
Kernel `6.6.87.1-microsoft-standard-WSL2`, glibc `2.41-12+deb13u3`, THP
`[madvise]`, THP defrag `[madvise]`. Corpus revision
`bdef075de28da656976c7cf54f89c1214dbc65d4` (2026-11-08 snapshot).

These numbers are host-specific observations, not portable performance
guarantees.

## Inventory

The pinned manifest contains 206 ARW files:

| Group | Files | Bodies | Layout |
| --- | ---: | ---: | --- |
| Uncompressed | 40 | 23 | 6 files at 12-bit strips, 34 at 14-bit strips |
| Lossless JPEG | 19 | 13 | 1 file at 12-bit 512x512 tiles, 18 at 14-bit 512x512 tiles |
| Excluded | 147 | 104 | ARW1, ARW2, ARW6, and 20 unknown/unsupported rows |

The excluded set is 8 ARW1 files, 115 ARW2 files, 4 ARW6 files, and 20 files
without a supported source encoding. No Sony ARW in this corpus reports a
16-bit source path.

All 19 `sony-lossless-jpeg` rows were checked by parsing the TIFF tile table and
the first JPEG marker stream. Every row starts with SOI (`ff d8`) and reaches
SOS through SOF3 (`ff c3`). The corpus therefore contains real lossless JPEG,
not a Sony-packed format named "lossless". Full evidence is in
[stream-verification.txt](arw-gap-profiles/stream-verification.txt).

The complete file-level inventory is
[inventory.csv](arw-gap-profiles/inventory.csv). It records the model,
compression tag, Sony raw-file type, bits, storage, tile size, geometry, file
size, decode support, and gate status for all 206 rows.

The timed representatives were:

| Case | Model | Compression | Sony raw-file type | Bits | Layout | Dimensions | File bytes |
| --- | --- | --- | --- | ---: | --- | --- | ---: |
| Uncompressed 14-bit | ILCE-7RM3 | `Compression=1` | Uncompressed 14-bit RAW | 14 | strip | 8000x5320 | 85,562,368 |
| LJPEG 12-bit | ZV-E10M2 | `Compression=7` | Lossless Compressed RAW 2 | 12 | 512x512 tiles | 6656x4608 | 32,579,584 |
| LJPEG 14-bit | ILCE-7RM5 | `Compression=7` | Lossless Compressed RAW 2 | 14 | 512x512 tiles | 9728x6656 | 85,950,464 |

`exiftool 13.63` confirmed the storage tags and bit depths on these
representatives; its selected fields are in
[exiftool-selected.json](arw-gap-profiles/exiftool-selected.json).

## Support Matrix

| Source case | Fornax | Rawler 0.8.0 | FlashRaw 0.2.0 | Three-way gate |
| --- | --- | --- | --- | --- |
| Uncompressed 14-bit ILCE-7RM3 | supported | supported | supported | pass at 1T and 15T |
| LJPEG 12-bit ZV-E10M2 | supported | supported | supported | pass at 1T and 15T |
| LJPEG 14-bit ILCE-7RM5 | supported | supported | supported | pass at 1T and 15T |
| Uncompressed 12-bit, five sampled bodies | decodes | decodes | decodes | fail against both references |

The five sampled 12-bit uncompressed files were ILCE-7RM2, ILCE-7RM3,
ILCE-7RM4, DSLR-A850, and DSLR-A900. Rawler and FlashRaw agree with each
other, while Fornax differs. Example: ILCE-7RM2 sample 0 is 3392 in Fornax and
1492 in both references. All dimensions, storage types, and bit declarations
match; pixel values do not, so those files cannot supply trusted ratios.

The remaining 51 rows retain their recorded decode support, but were not
re-gated after the user requested a representative measurement scope.

## Timings

Medians with min-max over 15 observations. Full method and table are in
[timings.txt](arw-gap-profiles/timings.txt).

| Case and threads | Fornax | Rawler | FlashRaw | Fastest | Fornax/FlashRaw | Fornax/Rawler |
| --- | ---: | ---: | ---: | --- | ---: | ---: |
| Uncompressed 14-bit, 1T | 247.555 ms | 196.940 ms | 30.129 ms | FlashRaw | 8.216x | 1.257x |
| Uncompressed 14-bit, 15T | 250.612 ms | 144.225 ms | 19.584 ms | FlashRaw | 12.797x | 1.738x |
| LJPEG 12-bit, 1T | 414.698 ms | 244.047 ms | 169.274 ms | FlashRaw | 2.450x | 1.699x |
| LJPEG 12-bit, 15T | 135.789 ms | 86.542 ms | 32.101 ms | FlashRaw | 4.230x | 1.569x |
| LJPEG 14-bit, 1T | 993.834 ms | 724.199 ms | 430.678 ms | FlashRaw | 2.308x | 1.372x |
| LJPEG 14-bit, 15T | 313.860 ms | 316.716 ms | 65.597 ms | FlashRaw | 4.785x | 0.991x |

Uncompressed 15T is flat because the raw source is a single strip/chunk.
Lossless JPEG has enough tiles to scale, but poor all-thread efficiency leaves
the FlashRaw gap larger at 15T than at 1T.

## Counters

Fornax and FlashRaw counters are normalized per decode from a five-decode
`perf stat` process after one unmeasured decode. Full values are in
[perf-stat.txt](arw-gap-profiles/perf-stat.txt).

| Case | Decoder | Instructions/sample | IPC | Branch misses/decode | Minor faults/decode |
| --- | --- | ---: | ---: | ---: | ---: |
| Uncompressed 14-bit, 1T | Fornax | 0.462 | 0.200 | 59,214 | 41,821 |
| Uncompressed 14-bit, 1T | FlashRaw | 0.220 | 0.306 | 15,930 | 2,243 |
| LJPEG 12-bit, 1T | Fornax | 187.312 | 4.415 | 3,382,922 | 18,374 |
| LJPEG 12-bit, 1T | FlashRaw | 62.499 | 3.046 | 250,971 | 1,255 |
| LJPEG 14-bit, 1T | Fornax | 199.255 | 4.303 | 7,614,862 | 52,818 |
| LJPEG 14-bit, 1T | FlashRaw | 63.797 | 2.607 | 2,724,646 | 2,302 |

Fornax's higher IPC reinforces that Sony LJPEG is instruction-volume bound.
Uncompressed Fornax executes few instructions but incurs about 19x FlashRaw's
minor faults, consistent with two full-source touches plus output first touch.

## Stage Split

Detailed one-thread measurements are in [stages.txt](arw-gap-profiles/stages.txt).

### Uncompressed

| Stage | ILCE-7RM3 14-bit |
| --- | ---: |
| Owned source read | 76.668 ms |
| TIFF/container parse after read | 74.796 ms |
| Output allocation and first touch | 54.402 ms |
| Pre-touched U16 row copy/unpack | 5.311 ms |

There is no Fornax source-mmap stage in this path. `fornax_tiff::parse` clones
the complete source through `source.to_vec`, and `ArwImage::parse` calls that
API. The ARW path therefore touches one complete input allocation for
`fs::read`, another for TIFF parsing, and then the U16 output. The source clone
is portable metadata/container overhead, not pixel decoding.

### Lossless JPEG

| Stage | ZV-E10M2 12-bit | ILCE-7RM5 14-bit |
| --- | ---: | ---: |
| TIFF/container parse | 11.796 ms | 150.304 ms |
| `parse_jpeg` over all tiles | 2.223 ms | 4.500 ms |
| Huffman construction, included in parse | 2.106 ms | 4.223 ms |
| Entropy scan | 293.939 ms | 686.082 ms |
| Residual, including tile/output allocation and destruction | 81.309 ms | 160.761 ms |

The 14-bit sony LJPEG stream has 247 tiles and pays substantial container cost
before entropy decoding starts. Within the pixel codec, Huffman construction
is not the gap; `parse_jpeg` plus Huffman is under 1% of the one-thread time.
The generic four-component entropy path owns the instruction gap.

## Bit-Depth Paths

| Source | Codec path | Fast-path status | Gate result |
| --- | --- | --- | --- |
| Uncompressed 14-bit | `codec_bits` maps 14 to 16; native-endian U16 row copy | fast copy path | pass |
| Uncompressed 16-bit | none in corpus | native-endian U16 copy is implemented | not applicable |
| Uncompressed A850/A900 12-bit | packed 12-bit path; `unpack_12_msb` | fast 12-bit unpacker | fail: values differ from both references |
| Uncompressed modern 12-bit | container declares 12 bits, but sampled files store two-byte words; Fornax still uses packed 12-bit | fast 12-bit unpacker, wrong interpretation for sampled files | fail: pixel 0 mismatch |
| Sony LJPEG 12/14-bit | SOF3, four components, generic interleaved scan | no DNG predictor-1 fast row path | pass |

The modern 12-bit mismatch is visible directly in ILCE-7RM2 sample 0. The two
stored bytes are `d4 05`, which are U16LE 1492, matching both references.
Fornax takes the first 12 bits of `d4 05 74`, producing 3392. This is a
bit-width/routing defect, not a slow-fallback claim.

## What Is Different From DNG

- DNG uncompressed was dominated by output first touch. ARW uncompressed adds a
  full owned source read and a second full-source clone in TIFF parsing before
  output allocation.
- DNG LJPEG reaches about 101 instructions/sample. Sony LJPEG is about
  187-199 instructions/sample because its four-component tiles use the generic
  interleaved decoder path and then a separate four-component reshape.
- DNG's fastest predictor-1 loop handles at most two components directly.
  Sony's four components cannot use that path.
- Sony's tile shape decodes four interleaved 256x256 component scans into a
  512x512 CFA tile. That layout requires both a full decoded tile buffer and a
  second output tile buffer in the current implementation.
- Huffman construction is materially larger relative to pixel work in DNG
  because DNG tiles can be numerous, but it is still not the dominant Sony ARW
  gap here.
- ARW parser metadata and private-IFD work are small relative to the source
  clone; they should not be conflated with entropy decoding.

## Unsupported Files

The baseline records 20 ARW files that Fornax does not currently decode. The
file-level rows and exact typed error are in
[inventory.csv](arw-gap-profiles/inventory.csv).

The unsupported set is:

- `SONY/DSC-RX1RM3/DSC00255.ARW`, `DSC00256.ARW`
- `SONY/ILCE-1M2/A1_07628_lossless_compressed_medium_full_frame.ARW`,
  `A1_07629_lossless_compressed_small_full_frame.ARW`,
  `A1_07633_lossless_compressed_small_apsc.ARW`
- `SONY/ILME-FX2/Image_4_-_Lossless_Compression_(M)_-_Shade.ARW`,
  `Image_5_-_Lossless_Compression_(S)_-_Shade.ARW`
- `Sony/DSLR-A100/_DSC0258.ARW` (`decode:invalid_input`)
- `Sony/ILCE-7CR/DSC00796.ARW`, `DSC00797.ARW`, `DSC00800.ARW`
- `Sony/ILCE-7M4/ILCE-7M4_DSC06675_FullFrame-LossLess-Compressed-Medium.ARW`,
  `DSC06676_FullFrame-LossLess-Compressed-Small.ARW`,
  `DSC06680_APS-C-LossLess-Small.ARW`
- `Sony/ILCE-7RM5/7RM5-LosslessCompressedMedium.ARW`,
  `7RM5-LosslessCompressedSmall.ARW`, `7RM5-S35-LosslessCompressedSmall.ARW`
- `Sony/ILCE-9M3/DSC00113.ARW`, `DSC00114.ARW`, `DSC00118.ARW`

Nineteen are recorded as `decode:unsupported`; DSLR-A100 is
`decode:invalid_input`. The Medium/Small rows are outside the full-resolution
CFA paths. The remaining unsupported rows need individual container-level
follow-up; none were used for timing.

## Portable Optimization Proposals

These are proposals, not implemented changes.

1. Remove the second full-source copy from the ARW parser. Parse the owned
   `TiffSource` directly instead of calling `fornax_tiff::parse(&[u8])`, which
   clones the complete file. This is portable scalar Rust and targets roughly
   75 ms of 1T uncompressed container time plus a full input's page faults.
2. Add a portable four-component Sony LJPEG kernel that predicts and writes
   the two CFA output rows directly, avoiding the generic interleaved result
   buffer, second tile buffer, and reshape. The evidence supports a smaller
   instruction stream; it does not support an SIMD requirement.

Before optimizing, the 12-bit uncompressed routing mismatch should be treated
as a correctness bug. Until it is fixed, 12-bit uncompressed performance
numbers are not meaningful.
