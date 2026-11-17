> Status: archived completed plan.
> Merged: #723 `chore/minimal-dng-golden` on 2026-11-15.
> Current guidance: [current documentation](../../reference/decoders/operations/corpus.md).

# Minimal DNG Golden Corpus

## Goal

Reduce DNG golden coverage to one representative for each implemented decoder
path while retaining strip/tile layout coverage.

## Scope

In scope:

- DNG-aware decoder-path grouping in `decoder-tracker`.
- A deterministic 13-file DNG golden set.
- Golden hashes, tests, documentation, and benchmark checks.

Out of scope:

- Native or storage-native `f16` changes.
- Codec, DNG transform, writer, and raw mosaic representation changes.
- Changes to ARW, CR3, NEF, or other non-DNG golden grouping.
- Full-corpus changes unrelated to DNG.

## Task Checklist

- [x] Add failing tracker tests for DNG decoder-path grouping and expected
      representatives.
- [x] Implement `decode_path` reporting and the minimal DNG selection.
- [x] Regenerate the golden manifest, hashes, and affected documentation.
- [x] Run tracker and selected cross-decoder checks.
- [x] Separate writer compression policy from codec compression and own the
      Zstandard level in the writer enum.
- [x] Add symmetric `ICodecEncode` dispatch and remove the compression field
      from `EncodeRequest`.
- [ ] Run the manual DNG corpus workflow and review its result.
- [ ] Run repository validation and final review.

## Expected DNG Representatives

- `Canon/EOS 60D/M14-1451_000085_cDNG_compressed.dng`
- `Canon/EOS 5D Mark III/M24-1221_000202.dng`
- `Canon/EOS 5D Mark III/hdrmerge-bayer-fp16-w-pred-deflate.dng`
- `Canon/EOS 5D Mark III/hdrmerge-bayer-fp24-w-pred-deflate.dng`
- `Canon/EOS 5D Mark III/hdrmerge-bayer-fp32-w-pred-deflate.dng`
- `Sigma/fp/A002_653_20240229_000001_2k_8bit.DNG`
- `Sigma/fp/A002_652_20240229_000001_2k_10bit.DNG`
- `Sigma/fp/A002_651_20240229_000001_2k_12bit.DNG`
- `Canon/PowerShot G1 X/CRW_0004.DNG`
- `Canon/Canon PowerShot S3IS/CRW_8531.DNG`
- `Canon/EOS 5D Mark III/hdrmerge-bayer-fp16-uncompressed.dng`
- `Canon/EOS 5D Mark III/hdrmerge-bayer-fp24-uncompressed.dng`
- `Canon/EOS 5D Mark III/hdrmerge-bayer-fp32-uncompressed.dng`

## Validation

```bash
cargo test -p decoder-tracker
cargo test -p decoder-tracker duckdb_
cargo run --release -q -p decoder-tracker -- hash \
  --manifest crates/decoder-tracker/golden/selected-manifest.csv \
  --check crates/decoder-tracker/golden/decode-hashes.csv
cargo test -p fornax-raw-decoder --release --test decoder_equivalence -- --ignored
cargo +nightly fmt --all --check
scripts/agent-validation.sh
```

After pushing the branch, run the manual decoder corpus workflow with the DNG
glob `**/*.[dD][nN][gG]`, review the selected coverage, and rerun until
unchanged.

## Decisions and Notes

- DNG Lossless JPEG groups across bit depths while retaining strip and tile
  layout.
- Uncompressed DNG integer groups follow the implemented u8, generic packed
  u16, u12, u14, and u16-copy kernels.
- Floating-point DNG widths remain separate for uncompressed and Deflate
  paths.
- Blackmagic packed-row JPEG geometry remains covered by codec and full-corpus
  tests, but not by the three-way benchmark golden set because FlashRaw scales
  Blackmagic `RawU16` precision.
- Native `f16` is deferred until Rust stabilizes it and the image buffer layer
  can support the sample type cleanly.
- `DngWriterCompression` owns writer policy and Zstandard parameters.
  `EncodeCompression` implements `ICodecEncode`, mirroring `DecodeCompression`
  and `ICodecDecode`.
