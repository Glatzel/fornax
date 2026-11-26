# Documentation

This index is the entry point for current Fornax architecture, decoder,
reference, and active planning documentation.

## How to navigate

- Read [architecture/overview.md](architecture/overview.md) first for crate
  boundaries and the end-to-end image pipeline.
- Use `reference/` as the canonical home for decoder operations, format support,
  pinned sources, algorithm/codec behavior, and implementation notes.
- Use `plans/` for work that is still active.
- Use [archive/README.md](archive/README.md) for completed plans and historical
  evidence. Archived documents are not current task instructions.
- Follow [CONVENTIONS.md](CONVENTIONS.md) when adding or moving documentation.

## Folder guide

| Folder | Contents |
| --- | --- |
| `architecture/` | Current architecture and data-flow guidance. |
| `reference/` | Canonical decoder operations and source research grouped by topic. |
| `plans/` | Active implementation plans for open work. |
| `archive/` | Completed plans, superseded reports, and reproducibility evidence. |

## Architecture

| Document | Summary |
| --- | --- |
| [Decoder Architecture](architecture/decoder.md) | Decoder dispatch, shared interfaces, and per-format data flow. |
| [Architecture Overview](architecture/overview.md) | Crate boundaries, stage contracts, and end-to-end image processing. |

## Decoder operations

| Document | Summary |
| --- | --- |
| [Decoder Benchmarks](reference/decoders/operations/benchmark.md) | Benchmark targets, environment controls, and comparison criteria. |
| [RAW Corpus and Regression Tracking](reference/decoders/operations/corpus.md) | Pinned corpus, golden regression workflow, schemas, and manual CI operation. |

## Active plans

| Document | Summary |
| --- | --- |
| [0.4.0 Release Readiness](plans/chore-0.4.0-release-readiness.md) | FlashRaw comparison refresh, supported-format golden coverage, and CI-owned payload validation. |

## Reference

### Algorithms and codecs

| Document | Summary |
| --- | --- |
| [Demosaic reference index](reference/demosaic/README.md) | Current implementations, retained provenance, and external source research. |
| [AAHD reference notes](reference/demosaic/aahd.md) | Pinned sources, numerical behavior, and implementation boundaries. |
| [AMaZE readiness](reference/demosaic/amaze.md) | Linear-light AMaZE equations, Nyquist/chroma fixes, and tile options. |
| [DCB reference notes](reference/demosaic/dcb.md) | Pinned sources, numerical behavior, and implementation boundaries. |
| [DHT readiness](reference/demosaic/dht.md) | Clean-room linear-light DHT equations, sign-safe green, and tile options. |
| [Linear readiness](reference/demosaic/linear.md) | Bilinear reconstruction, border behavior, and fixed snapshot coverage. |
| [MHC Tier -1 research](reference/demosaic/mhc.md) | Source equations and the reference-only patent/license disposition. |
| [LMMSE readiness](reference/demosaic/lmmse.md) | Linear-light Zhang-Wu LMMSE equations, tile options, and verification. |
| [PPG readiness](reference/demosaic/ppg.md) | Modern formulation provenance, equations, readiness, and fixed snapshot coverage. |
| [RawTherapee reference notes](reference/demosaic/rawtherapee.md) | Pinned sources, numerical behavior, and implementation boundaries. |
| [RCD readiness](reference/demosaic/rcd.md) | Linear-light RCD 2.3 equations, signed zero-crossing rule, and tile options. |
| [VNG and VNG4 readiness](reference/demosaic/vng.md) | Linear-light 3-color VNG and 4-color split-green VNG4 equations and verification. |
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
| [Other RAW format policy](reference/decoders/reference/other-formats.md) | Unsupported-format boundary and requirements for adding a format. |
| [Panasonic RW2 references](reference/decoders/reference/panasonic-rw2.md) | Pinned RW2 sources and implementation boundaries. |
| [Phase One IIQ references](reference/decoders/reference/phase-one-iiq.md) | Pinned IIQ sources and implementation boundaries. |
| [Restart-free single-scan parallelism design](reference/decoders/design/restart-free-single-scan-parallelism-design.md) | Written no-go gate for restart-free LJPEG parallelism. |
| [Sony ARW/NRW reference notes](reference/decoders/reference/sony-arw.md) | Pinned ARW/NRW sources and implementation boundaries. |

## Documentation conventions

| Document | Summary |
| --- | --- |
| [Documentation Conventions](CONVENTIONS.md) | Documentation locations, naming, structure, and archive policy. |
