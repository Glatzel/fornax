# Decoder hash cleanup and uncompressed row performance

## Goal

Resolve stale Sony uncompressed hashes, close Nikon uncompressed performance-set
coverage, make the 34-file performance manifest practical to run in groups, and
finish the Sony/Nikon uncompressed-word path with mapped, row-parallel direct
decode while preserving every accepted raw hash.

## Scope

In scope:

- Three stale Sony uncompressed baseline rows and one additional test guard.
- Nikon D500/D850 uncompressed-word path classification and perf coverage.
- Perf-manifest `core`, `full`, `overhead`, and `hdr` groups.
- Sony 16-bit-word and Nikon uncompressed-word input mapping, direct row
  unpacking, row parallelism, strict truncation handling, tests, and benchmarks.
- Concise results in the existing FlashRaw comparison report.

Out of scope:

- Packed Sony/Nikon paths, Huffman, LJPEG, CRX, lossy modes, SIMD, new unsafe,
  public API changes, or unrelated DNG kernels.

## Tasks

- [x] Resolve the three Sony baseline hashes with Fornax, FlashRaw, and Rawler.
- [x] Add one Sony uncompressed regression file to the test manifest and shared
  hashes.
- [x] Classify D500/D850 uncompressed NEF layout and add stable perf coverage
  only if it is not already represented.
- [x] Add perf-manifest groups and grouped benchmark/checker selection.
- [x] Confirm current Sony/Nikon read, copy, allocation, and parallelism paths.
- [x] Capture interleaved before measurements, including faults and RSS.
- [x] Complete mapped, strict, row-parallel direct unpacking and focused tests.
- [x] Verify all uncompressed ARW/NEF hashes and the full golden set.
- [x] Run full repository validation.
- [x] Capture interleaved after measurements and profile any remaining gap.
- [x] Update the existing FlashRaw comparison report only with the current
  final result summary.

## Files expected to change

- `crates/decoder-tracker/data/baseline.csv`
- `crates/decoder-tracker/golden/{test,perf}-manifest.csv`
- `crates/decoder-tracker/golden/decode-hashes.csv`
- `crates/decoder-tracker/src/{golden_set.rs,main.rs,csvio.rs}`
- decoder benchmark input/selection code and the FlashRaw probe
- `crates/fornax-codec/src/samples.rs`
- `crates/fornax-raw-sony/*`
- `crates/fornax-raw-nikon/*`
- decoder/corpus/benchmark documentation and this plan

## Validation

- `cargo test -p decoder-tracker`
- `cargo test -p fornax-codec`
- `cargo test -p fornax-raw-sony`
- `cargo test -p fornax-raw-nikon`
- `cargo test -p fornax-raw-decoder`
- golden-check and golden hash verification
- interleaved 1T/16T A/B probe with controls, faults, and RSS
- `scripts/agent-validation.sh`

## Decisions and notes

- The current branch already routes Sony and Nikon TIFF containers through
  `TiffSource::from_path` and has a row-band `unpack_u16_word_rows` helper.
  Work starts by proving whether additional changes are needed, not by
  duplicating those facilities.
- The retained changes raise the row-parallel threshold to four million samples
  and use the existing large-allocation helper for the Nikon one-thread words
  path. At 16 threads Nikon keeps a plain zeroed Vec because huge-page advisory
  and first-touch contention measured substantially slower there.
- `perf stat` on ILCE-1 at 16 threads reports 19.14% cache misses and only 5.4
  CPUs utilized. `perf record` attributes most post-hash samples to libc memory
  movement; the remaining gap is page residency, memory bandwidth, and
  scheduling rather than arithmetic.
- `external/raw.pixls.us` is locally modified by LFS materialization and is
  excluded from every commit.
- RAW payloads remain transient and uncommitted.
