# DNG Performance Resume Notes

This page is the compact restart point for DNG and LJPEG performance work.

Read [the current gap report](../dng-remaining-gap.md) first, then
[the experiment log](../dng-experiments-log.md), the
[FlashRaw CPU comparison](../flashraw-cpu-comparison.md), and the
[restart-free parallelism design](../../design/restart-free-single-scan-parallelism-design.md).
The signed-off rule is that FlashRaw 0.2.0 is MIT OR Apache-2.0 and is a source
of ideas only unless copied code is separately identified and licensed.

## Probes

| Probe | Purpose |
| --- | --- |
| [`ljpeg_gap_probe.rs`](../../../../../crates/fornax-codec/benches/ljpeg_gap_probe.rs) | Generates smooth/noisy one- and two-component LJPEG scans with configurable tile counts. |
| [`dng_gap_probe.rs`](../../../../../crates/fornax-raw-decoder/benches/dng_gap_probe.rs) | Dispatches Linux equality, decode, allocator, huge-page, and fault probes. |
| [`dng_gap_probe/linux.rs`](../../../../../crates/fornax-raw-decoder/benches/dng_gap_probe/linux.rs) | Implements exact three-way equality, decode timing, `AnonHugePages`, and allocator variants. |
| [`flashraw_wrapper.rs`](../../../../../crates/fornax-raw-decoder/benches/flashraw_wrapper.rs) | Forces FlashRaw 0.2.0 through scalar `Backend::Cpu`. |
| [`decoder.rs`](../../../../../crates/fornax-raw-decoder/benches/decoder.rs) | Runs the checked-in decoder benchmark matrix. |

## Rebuild and rerun

```bash
CARGO_PROFILE_RELEASE_DEBUG=line-tables-only \
  cargo build -p fornax-raw-decoder --release --bench dng_gap_probe

cargo build -p fornax-codec --release --bench ljpeg_gap_probe
```

Run the exact equality gate before timing:

```bash
RAYON_NUM_THREADS=1 \
  cargo bench -p fornax-raw-decoder --bench dng_gap_probe -- \
  equality 'external/raw.pixls.us/Adobe DNG Converter/Canon EOS 5D Mark III/5G4A9394-compressed-lossless.DNG'
```

Run a same-session Fornax/FlashRaw comparison with one or fifteen workers:

```bash
RAYON_NUM_THREADS=1 FORNAX_FLASHRAW_THREADS=1 \
  cargo bench -p fornax-raw-decoder --bench dng_gap_probe -- \
  decode --decoder fornax --warmups 1 --rounds 5 INPUT

RAYON_NUM_THREADS=1 FORNAX_FLASHRAW_THREADS=1 \
  cargo bench -p fornax-raw-decoder --bench dng_gap_probe -- \
  decode --decoder flashraw --warmups 1 --rounds 5 INPUT
```

Use `RAYON_NUM_THREADS=15`, `--threads all`, and
`FORNAX_FLASHRAW_THREADS=all` for the all-thread case. Use `perf stat` for
cycles, instructions, branch misses, and L1D misses. Confirm
`/sys/kernel/mm/transparent_hugepage/enabled`, and use the probe's `--smaps`
option to verify `AnonHugePages` before claiming a huge-page effect.

## Inputs

Paths are relative to `external/raw.pixls.us` or
`external/raw-images`.

| Case | Input | SHA-256 |
| --- | --- | --- |
| 5D3 uncompressed | `Adobe DNG Converter/Canon EOS 5D Mark III/5G4A9394-uncompressed.DNG` | `95dc76a2f255e623acaa5149b320389f7da7c995da890c3e0d511c0ff681b2e1` |
| 5D3 LJPEG | `Adobe DNG Converter/Canon EOS 5D Mark III/5G4A9394-compressed-lossless.DNG` | `3118116735d4f6dd01fd9d6a80ee9aac52a2c2debdc6dcd112bd5e3059044a55` |
| Sigma fp 4K | `Sigma/fp/A002_647_20240229_000001_4k_12bit.DNG` | `2f28f7037bf123012174285a566555c833acc7bd9f0b43dee99bb603824956d3` |
| Pixel 8 Pro LJPEG | `Google/Pixel 8 Pro/PXL_20240415_103400204.RAW-02.ORIGINAL.dng` | `45420c595401547cca950ae58d552bc82b32d31ce1d58c082df33004016197f5` |
| iPhone SE LJPEG | `Apple/iPhone SE/A4973FFB-9CBD-4ED8-805D-E30F4AE08A95.dng` | `7a2a9747a0cb1537007233ce7e8b7233c5ee641d683b7b3da29e22387994a0d7` |
| Pentax K-5 LJPEG | `Pentax/K-5/_MAR0543.DNG` | `6f6419209340d07d597ccca66dc367d64cff3308ef2694c8f90c6259f3a5c431` |
| 5D3 HDR FP16 Deflate | `Canon/EOS 5D Mark III/hdrmerge-bayer-fp16-w-pred-deflate.dng` | `6275226bfa2d86ba479130999d045ea0b8da6be4b011130c9eb91cd6a0eb2233` |
| 5D3 HDR FP32 Deflate | `Canon/EOS 5D Mark III/hdrmerge-bayer-fp32-w-pred-deflate.dng` | `94993fefdac13a68a40f0c4f47479256ff53790d8af8ad01ecf6b78d7d2b553f` |

Baseline commits are `eaf7ec6` for the post-#738 allocator state,
`985c704` for the #736 portable scalar state, `65cbb38` for the #734
portable baseline, and `1271446` for the original fused predictor pass.

## Open items

- Caller-owned `decode_into` remains a proposal; do not implement it without a
  separate API decision.
- Re-measure on native Linux, Windows, and macOS; the current numbers are
  WSL2/Linux-only.
- Continue the Sony ARW gap investigation from
  [the ARW report](../arw-remaining-gap.md).
- Design and measure interleaving two tile chains only as a separate scalar
  experiment.
- Reconsider SIMD only after the scalar path and its evidence remain stable.
