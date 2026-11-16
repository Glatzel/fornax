# Documentation

This index is the entry point for current Fornax architecture, decoder,
reference, and active planning documentation.

## How to navigate

- Read [architecture/overview.md](architecture/overview.md) first for crate
  boundaries and the end-to-end image pipeline.
- Use `decoder/` for decoder dispatch, format support, benchmarks, and corpus
  regression operations.
- Use `reference/` for pinned sources, algorithm behavior, codec ownership,
  performance evidence, and format-specific implementation notes.
- Use `plans/` for work that is still active.
- Use [archive/README.md](archive/README.md) for completed plans and historical
  evidence. Archived documents are not current task instructions.
- Follow [CONVENTIONS.md](CONVENTIONS.md) when adding or moving documentation.

## Folder guide

| Folder | Contents |
| --- | --- |
| `architecture/` | Current architecture and data-flow guidance. |
| `decoder/` | Decoder operations plus format-level implementation and support plans. |
| `reference/` | Durable source research grouped by algorithm, codec, and decoder. |
| `plans/` | Active implementation plans for open work. |
| `archive/` | Completed plans, superseded reports, and reproducibility evidence. |

## Architecture

| Document | Summary |
| --- | --- |
| [Decoder Architecture](architecture/decoder.md) | Decoder dispatch, shared interfaces, and per-format data flow. |
| [Architecture Overview](architecture/overview.md) | Crate boundaries, stage contracts, and end-to-end image processing. |

## Decoder guides

| Document | Summary |
| --- | --- |
| [Decoder Benchmarks](decoder/benchmark.md) | Benchmark targets, environment controls, and comparison criteria. |
| [RAW Corpus and Regression Tracking](decoder/corpus.md) | Pinned corpus, golden regression workflow, schemas, and manual CI operation. |
| [Canon CR3 Format](decoder/formats/canon-cr3.md) | Canon CR3/CRM container, compression, metadata, and completion requirements. |
| [DNG Format](decoder/formats/dng.md) | DNG container, compression, metadata, and completion requirements. |
| [Fujifilm RAF Format](decoder/formats/fuji-raf.md) | RAF container, compression, metadata, and completion requirements. |
| [Hasselblad 3FR Format](decoder/formats/hasselblad-3fr.md) | 3FR container, compression, metadata, and completion requirements. |
| [Nikon NEF Format](decoder/formats/nikon-nef.md) | NEF container, compression, metadata, and completion requirements. |
| [Other RAW Formats Policy](decoder/formats/other-formats.md) | Unsupported-format boundary and requirements for adding a format. |
| [Panasonic RW2 Format](decoder/formats/panasonic-rw2.md) | RW2 container, compression, metadata, and completion requirements. |
| [Phase One IIQ Format](decoder/formats/phase-one-iiq.md) | IIQ container, compression, metadata, and completion requirements. |
| [Sony ARW Format](decoder/formats/sony-arw.md) | ARW/NRW container, compression, metadata, and completion requirements. |

## Active plans

| Document | Summary |
| --- | --- |
| [Shareable Book PDF Build](plans/book-03-pdf-build.md) | Remaining work for a shareable book PDF build. |
| [Compare-Demosaic Reference Algorithms](plans/compare-demosaic-algorithms.md) | Completed and pending reference-algorithm port tasks. |

## Reference

### Algorithms and codecs

| Document | Summary |
| --- | --- |
| [AAHD reference notes](reference/algorithms/aahd.md) | Pinned sources, numerical behavior, and implementation boundaries. |
| [AMaZE reference notes](reference/algorithms/amaze.md) | Pinned sources, numerical behavior, and implementation boundaries. |
| [DCB reference notes](reference/algorithms/dcb.md) | Pinned sources, numerical behavior, and implementation boundaries. |
| [DHT reference notes](reference/algorithms/dht.md) | Pinned sources, numerical behavior, and implementation boundaries. |
| [LMMSE reference notes](reference/algorithms/lmmse.md) | Pinned sources, numerical behavior, and implementation boundaries. |
| [RawTherapee reference notes](reference/algorithms/rawtherapee.md) | Pinned sources, numerical behavior, and implementation boundaries. |
| [VNG reference notes](reference/algorithms/vng.md) | Pinned sources, numerical behavior, and implementation boundaries. |
| [TIFF-Family Compression ID Ownership](reference/codecs/compression-ids.md) | Ownership of TIFF-family compression identifiers. |
| [Corpus Losslessness Classification](reference/codecs/lossless-classification.md) | Provenance and conservative corpus compression boundaries. |

### Decoders

| Document | Summary |
| --- | --- |
| [Decoder references](reference/decoders/README.md) | Index of decoder performance, design, and format/API research. |
| [FlashRaw 0.3.1 re-measurement](reference/decoders/perf/flashraw-0.3.1-remeasure-2026-11-16.md) | Lossless DNG/ARW/CR3/NEF comparison, pipeline profile, and ranked follow-ups. |
| [Sony ARW remaining performance gap](reference/decoders/perf/arw-remaining-gap.md) | ARW encoding inventory, timings, and container/entropy costs. |
| [Canon CRX reference notes](reference/decoders/reference/canon-crx.md) | Canon CRX sources, behavior, and implementation boundaries. |
| [DNG compression references](reference/decoders/reference/dng-compression.md) | DNG compression sources and behavior. |
| [DNG and lossless JPEG-92 reference notes](reference/decoders/reference/dng-lossless-jpeg.md) | DNG and lossless JPEG-92 sources and implementation boundaries. |
| [DNG opcode reference notes](reference/decoders/reference/dng-opcodes.md) | DNG opcode sources and behavior. |
| [DNG output color reference notes](reference/decoders/reference/dng-output-color.md) | Source matrices, profile selection, and output color. |
| [DNG performance experiment log](reference/decoders/perf/dng-experiments-log.md) | Kept and reverted DNG experiments with evidence and decisions. |
| [DNG decode remaining performance gap](reference/decoders/perf/dng-remaining-gap.md) | Final DNG timings, remaining FlashRaw gap, and evidence limits. |
| [DNG performance resume notes](reference/decoders/perf/dng-gap-profiles/RESUME.md) | Commands, probes, input hashes, baseline commits, and open items. |
| [DNG performance profile index](reference/decoders/perf/dng-gap-profiles/README.md) | Reproduction commands, baseline setup, and current evidence tables. |
| [FlashRaw CPU source comparison](reference/decoders/perf/flashraw-cpu-comparison.md) | Scalar CPU design comparison and ranked hypotheses. |
| [FlashRaw reference notes](reference/decoders/reference/flashraw.md) | Pinned benchmark adapter, neutral settings, and decoder differences. |
| [Fujifilm RAF references](reference/decoders/reference/fuji-raf.md) | Pinned RAF sources and implementation boundaries. |
| [Hasselblad 3FR references](reference/decoders/reference/hasselblad-3fr.md) | Pinned 3FR sources and implementation boundaries. |
| [Nikon NEF references](reference/decoders/reference/nikon-nef.md) | Pinned NEF sources and implementation boundaries. |
| [Panasonic RW2 references](reference/decoders/reference/panasonic-rw2.md) | Pinned RW2 sources and implementation boundaries. |
| [Phase One IIQ references](reference/decoders/reference/phase-one-iiq.md) | Pinned IIQ sources and implementation boundaries. |
| [Restart-free single-scan parallelism design](reference/decoders/design/restart-free-single-scan-parallelism-design.md) | Written no-go gate for restart-free LJPEG parallelism. |
| [Sony ARW/NRW reference notes](reference/decoders/reference/sony-arw.md) | Pinned ARW/NRW sources and implementation boundaries. |

## Documentation conventions

| Document | Summary |
| --- | --- |
| [Documentation Conventions](CONVENTIONS.md) | Documentation locations, naming, structure, and archive policy. |
