# Documentation

This index is the entry point for current Fornax architecture, decoder,
reference, and active planning documentation.

## How to navigate

- Read [architecture/overview.md](architecture/overview.md) first for crate
  boundaries and the end-to-end image pipeline.
- Use `decoder/` for decoder dispatch, compact format support summaries,
  benchmarks, and corpus regression operations.
- Use `reference/` as the canonical home for pinned sources, algorithm and
  codec behavior, and detailed format implementation notes.
- Use `plans/` for work that is still active.
- Use [archive/README.md](archive/README.md) for completed plans and historical
  evidence. Archived documents are not current task instructions.
- Follow [CONVENTIONS.md](CONVENTIONS.md) when adding or moving documentation.

## Folder guide

| Folder | Contents |
| --- | --- |
| `architecture/` | Current architecture and data-flow guidance. |
| `decoder/` | Decoder operations plus format support summaries and acceptance gates. |
| `reference/` | Canonical source research grouped by algorithm, codec, and decoder. |
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
| [RAW Format Support](decoder/formats/README.md) | Supported layouts, rejection boundaries, and per-format acceptance gates. |
| [Canon CR3 Format](decoder/formats/canon-cr3.md) | Lossless CRX support boundary and acceptance gate. |
| [DNG Format](decoder/formats/dng.md) | Tier-0 DNG support summary and remaining limits. |
| [Fujifilm RAF Format](decoder/formats/fuji-raf.md) | Lossless RAF support and Bayer/X-Trans boundary. |
| [Hasselblad 3FR Format](decoder/formats/hasselblad-3fr.md) | Accepted 3FR profiles and codec boundary. |
| [Nikon NEF Format](decoder/formats/nikon-nef.md) | Nikon compression-mode support matrix. |
| [Other RAW Formats Policy](decoder/formats/other-formats.md) | Unsupported-format boundary and requirements for adding a format. |
| [Panasonic RW2 Format](decoder/formats/panasonic-rw2.md) | Accepted single-shot RW2 versions and codec boundary. |
| [Phase One IIQ Format](decoder/formats/phase-one-iiq.md) | Accepted IIQ modes and correction/acceptance boundary. |
| [Sony ARW Format](decoder/formats/sony-arw.md) | Accepted ARW/NRW families and rejection boundary. |

## Active plans

| Document | Summary |
| --- | --- |
| [Shareable Book PDF Build](plans/book-03-pdf-build.md) | Remaining work for a shareable book PDF build. |
| [Compare-Demosaic Reference Algorithms](plans/compare-demosaic-algorithms.md) | Completed and pending reference-algorithm port tasks. |
| [Clean decoder performance docs](plans/chore-clean-decoder-performance-docs.md) | Consolidate the active decoder performance set and preserve historical evidence in the archive. |

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
| [FlashRaw versus Fornax raw decode](reference/decoders/perf/flashraw-vs-fornax.md) | Canonical golden-corpus coverage, per-path timings, and ranked decoder improvements. |
| [Linux large-page output allocation](reference/decoders/design/large-page-allocation.md) | Owned-output allocation policy, invariants, and rejected alternatives. |
| [Canon CRX reference notes](reference/decoders/reference/canon-crx.md) | Canon CRX sources, behavior, and implementation boundaries. |
| [DNG decoder reference notes](reference/decoders/reference/dng-decoder.md) | Canonical DNG storage, metadata, CFA, and acceptance behavior. |
| [DNG compression references](reference/decoders/reference/dng-compression.md) | DNG compression sources and behavior. |
| [DNG and lossless JPEG-92 reference notes](reference/decoders/reference/dng-lossless-jpeg.md) | DNG and lossless JPEG-92 sources and implementation boundaries. |
| [DNG opcode reference notes](reference/decoders/reference/dng-opcodes.md) | DNG opcode sources and behavior. |
| [DNG output color reference notes](reference/decoders/reference/dng-output-color.md) | Source matrices, profile selection, and output color. |
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
