# Decoder References

Current decoder documentation is grouped by primary purpose:

- [operations](operations/) contains benchmark and corpus regression workflows.
- [perf](perf/) contains the authoritative cross-decoder performance report and
  its reproducible input/results data.
- [design](design/) contains active allocation and concurrency design
  boundaries.
- [reference](reference/) contains format, decoder, API, field-table, and
  specification research, including each format's support and acceptance gate.

## Operations

- [Decoder benchmarks](operations/benchmark.md)
- [RAW corpus and regression tracking](operations/corpus.md)

## Performance

- [FlashRaw versus Fornax raw decode](perf/flashraw-vs-fornax.md) is the single
  current comparison for the golden DNG, Sony ARW, Canon CR3, and Nikon NEF
  decoder paths.
- [Golden test manifest](../../../crates/decoder-tracker/golden/test-manifest.csv)
- [Golden perf manifest](../../../crates/decoder-tracker/golden/perf-manifest.csv)
- [Golden performance matrix](perf/data/golden-benchmark-matrix.csv)
- [Canonical per-sample results](perf/data/golden-flashraw-vs-fornax-2026-11-16.csv)

Historical FlashRaw 0.2.0/0.3.1 reports, DNG experiment logs, Sony ARW
inventories, raw profile artifacts, and negative-result evidence are in the
[decoder performance archive](../../archive/reference/decoders/perf/). These
are provenance, not current instructions.

## Design

- [Linux large-page output allocation](design/large-page-allocation.md)
- [Restart-free single-scan LJPEG parallelism](design/restart-free-single-scan-parallelism-design.md)

## Reference

- [Canon CRX and CR3](reference/canon-crx.md)
- [DNG decoder](reference/dng-decoder.md)
- [DNG compression](reference/dng-compression.md)
- [DNG and lossless JPEG-92](reference/dng-lossless-jpeg.md)
- [DNG opcodes](reference/dng-opcodes.md)
- [DNG output color](reference/dng-output-color.md)
- [FlashRaw benchmark oracle](reference/flashraw.md)
- [Fujifilm RAF](reference/fuji-raf.md)
- [Hasselblad 3FR](reference/hasselblad-3fr.md)
- [Nikon NEF](reference/nikon-nef.md)
- [Other RAW format policy](reference/other-formats.md)
- [Panasonic RW2](reference/panasonic-rw2.md)
- [Phase One IIQ](reference/phase-one-iiq.md)
- [Sony ARW/NRW](reference/sony-arw.md)
