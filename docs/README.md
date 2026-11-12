# Documentation

This index is the entry point for Fornax architecture, decoder, reference, and active planning documentation.

## How to navigate

- Read [architecture/overview.md](architecture/overview.md) first for crate
  boundaries and the end-to-end image pipeline.
- Use `decoder/` for decoder dispatch, format support, benchmarks, and corpus
  regression operations.
- Use `reference/` for pinned external sources, algorithm behavior, codec
  ownership, and format-specific implementation evidence.
- Use `plans/` for plans for work that is still active.
- Treat `_archive/` as historical evidence only. Archived plans are not
  current task instructions.
- Follow [CONVENTIONS.md](CONVENTIONS.md) when adding or moving documentation.

## Folder guide

| Folder | Contents |
| --- | --- |
| `architecture/` | Current architecture and data-flow guidance. |
| `decoder/` | Decoder operations plus format-level implementation and support plans. |
| `reference/` | Durable source research grouped by algorithm, codec, and decoder. |
| `plans/` | Active implementation plans for open work. |
| `_archive/` | Completed plans, workspace-refactor records, and design provenance. |

## Complete index

### Architecture

| Document | Summary |
| --- | --- |
| [Decoder Architecture](architecture/decoder.md) | This document explains decoder dispatch, shared interfaces, and per-format data flow. |
| [Architecture Overview](architecture/overview.md) | This document explains Fornax's crate boundaries, stage contracts, and end-to-end image-processing data flow. |

### Decoder guides

| Document | Summary |
| --- | --- |
| [Decoder Benchmarks](decoder/benchmark.md) | This guide documents decoder benchmark targets, environment controls, and comparison criteria. |
| [RAW Corpus and Regression Tracking](decoder/corpus.md) | This guide documents the pinned RAW corpus, golden regression workflow, schemas, and manual CI operation. |
| [Canon CR3 Format](decoder/formats/canon-cr3.md) | This format guide defines the supported container, compression, metadata, and completion requirements for Canon CR3. |
| [DNG Format](decoder/formats/dng.md) | This format guide defines the supported container, compression, metadata, and completion requirements for DNG. |
| [Fujifilm RAF Format](decoder/formats/fuji-raf.md) | This format guide defines the supported container, compression, metadata, and completion requirements for Fujifilm RAF. |
| [Hasselblad 3FR Format](decoder/formats/hasselblad-3fr.md) | This format guide defines the supported container, compression, metadata, and completion requirements for Hasselblad 3FR. |
| [Nikon NEF Format](decoder/formats/nikon-nef.md) | This format guide defines the supported container, compression, metadata, and completion requirements for Nikon NEF. |
| [Other RAW Formats Policy](decoder/formats/other-formats.md) | This policy defines the explicit unsupported-format boundary and the requirements for adding a new RAW format. |
| [Panasonic RW2 Format](decoder/formats/panasonic-rw2.md) | This format guide defines the supported container, compression, metadata, and completion requirements for Panasonic RW2. |
| [Phase One IIQ Format](decoder/formats/phase-one-iiq.md) | This format guide defines the supported container, compression, metadata, and completion requirements for Phase One IIQ. |
| [Sony ARW Format](decoder/formats/sony-arw.md) | This format guide defines the supported container, compression, metadata, and completion requirements for Sony ARW. |

### Active plans

| Document | Summary |
| --- | --- |
| [Shareable Book PDF Build](plans/book-03-pdf-build.md) | This active plan defines the remaining work for a shareable book PDF build. |
| [Documentation Reorganization Implementation Plan](plans/chore-reorganize-docs.md) | This plan records the approved documentation reorganization and its validation steps. |
| [Compare-Demosaic Reference Algorithms](plans/compare-demosaic-algorithms.md) | This active plan records completed and pending reference-algorithm port tasks for the demosaic comparison suite. |

### Reference

| Document | Summary |
| --- | --- |
| [AAHD reference notes](reference/algorithms/aahd.md) | This reference note records pinned sources, numerical behavior, and implementation boundaries for AAHD. |
| [AMaZE reference notes](reference/algorithms/amaze.md) | This reference note records pinned sources, numerical behavior, and implementation boundaries for AMaZE. |
| [DCB reference notes](reference/algorithms/dcb.md) | This reference note records pinned sources, numerical behavior, and implementation boundaries for DCB. |
| [DHT reference notes](reference/algorithms/dht.md) | This reference note records pinned sources, numerical behavior, and implementation boundaries for DHT. |
| [LMMSE reference notes](reference/algorithms/lmmse.md) | This reference note records pinned sources, numerical behavior, and implementation boundaries for LMMSE. |
| [RawTherapee Reference Notes](reference/algorithms/rawtherapee.md) | This reference note records pinned sources, numerical behavior, and implementation boundaries for RawTherapee. |
| [VNG reference notes](reference/algorithms/vng.md) | This reference note records pinned sources, numerical behavior, and implementation boundaries for VNG. |
| [TIFF-Family Compression ID Ownership](reference/codecs/compression-ids.md) | This reference note records ownership of TIFF-family compression identifiers and the rules that separate generic TIFF vocabulary from DNG-specific values. |
| [Canon CRX reference notes](reference/decoders/canon-crx.md) | This reference note records pinned sources, decoder behavior, and implementation boundaries for Canon CRX. |
| [DNG compression references](reference/decoders/dng-compression.md) | This reference note records pinned sources, decoder behavior, and implementation boundaries for DNG compression. |
| [DNG and lossless JPEG-92 reference notes](reference/decoders/dng-lossless-jpeg.md) | This reference note records pinned sources, decoder behavior, and implementation boundaries for DNG and lossless JPEG-92. |
| [DNG opcode reference notes](reference/decoders/dng-opcodes.md) | This reference note records pinned sources, decoder behavior, and implementation boundaries for DNG opcode. |
| [DNG Output Color Management](reference/decoders/dng-output-color.md) | This reference note documents source matrices, profile selection, and Fornax's DNG output color transform. |
| [Fujifilm RAF references](reference/decoders/fuji-raf.md) | This reference note records pinned sources, decoder behavior, and implementation boundaries for Fujifilm RAF. |
| [Hasselblad 3FR reference notes](reference/decoders/hasselblad-3fr.md) | This reference note records pinned sources, decoder behavior, and implementation boundaries for Hasselblad 3FR. |
| [Nikon NEF reference notes](reference/decoders/nikon-nef.md) | This reference note records pinned sources, decoder behavior, and implementation boundaries for Nikon NEF. |
| [Panasonic RW2 references](reference/decoders/panasonic-rw2.md) | This reference note records pinned sources, decoder behavior, and implementation boundaries for Panasonic RW2. |
| [Phase One/Leaf IIQ references](reference/decoders/phase-one-iiq.md) | This reference note records pinned sources, decoder behavior, and implementation boundaries for Phase One/Leaf IIQ. |
| [Sony ARW/NRW reference notes](reference/decoders/sony-arw.md) | This reference note records pinned sources, decoder behavior, and implementation boundaries for Sony ARW/NRW. |

### Archived implementation plans

| Document | Summary |
| --- | --- |
| [Agent Guidance Refresh Implementation Plan](_archive/plans/agent-guidance-refresh.md) | This archived plan documents the completed Agent Guidance Refresh work. |
| [Agent Guidance and Architecture Documentation Revision](_archive/plans/agents-guidance-revision.md) | This archived plan documents the completed Agent Guidance and Architecture Documentation Revision work. |
| [AMaZE faithful-port implementation plan](_archive/plans/amaze-port.md) | This archived plan documents the completed AMaZE faithful-port work. |
| [Merge Benchmark Crates Implementation Plan](_archive/plans/benchmark-crate-merge.md) | This archived plan documents the completed Merge Benchmark Crates work. |
| [Human-Facing Book Bootstrap](_archive/plans/book-bootstrap.md) | This archived plan documents the completed Human-Facing Book Bootstrap work. |
| [Procedural Book Figure Generation](_archive/plans/book-figures.md) | This archived plan documents the completed Procedural Book Figure Generation work. |
| [Canon CR3/CRM Lossless Decoder](_archive/plans/canon-cr3-decoder.md) | This archived plan documents the completed Canon CR3/CRM Lossless Decoder work. |
| [CI Path-Based Job Gating Implementation Plan](_archive/plans/ci-change-detection.md) | This archived plan documents the completed CI Path-Based Job Gating work. |
| [Simplify Codec and Dependency APIs](_archive/plans/codec-api-simplification.md) | This archived plan documents the completed Simplify Codec and Dependency APIs work. |
| [Codec Enum API Implementation Plan](_archive/plans/codec-enum-api.md) | This archived plan documents the completed Codec Enum API work. |
| [Add neutral codec sample buffer](_archive/plans/codec-sample-buffer.md) | This archived plan documents the completed Add neutral codec sample buffer work. |
| [Eliminate uncompressed and lossless JPEG writer regressions](_archive/plans/codec-writer-performance.md) | This archived plan documents the completed Eliminate uncompressed and lossless JPEG writer regressions work. |
| [DCB Faithful Port Implementation Plan](_archive/plans/dcb-port.md) | This archived plan documents the completed DCB Faithful Port work. |
| [Decoder Benchmark and Devtool Controls Implementation Plan](_archive/plans/decoder-benchmark-controls.md) | This archived plan documents the completed Decoder Benchmark and Devtool Controls work. |
| [Manual DNG Corpus Regression Tracker](_archive/plans/decoder-corpus-tracker.md) | This archived plan documents the completed Manual DNG Corpus Regression Tracker work. |
| [Decoder Tracker Agent Guide Implementation Plan](_archive/plans/decoder-tracker-agent-guide.md) | This archived plan documents the completed Decoder Tracker Agent Guide work. |
| [Demosaic linear workflow and parameter audit](_archive/plans/demosaic-parameter-audit.md) | This archived plan documents the completed Demosaic linear workflow and parameter audit work. |
| [Move Demosaic Protocol Traits Implementation Plan](_archive/plans/demosaic-protocol-types.md) | This archived plan documents the completed Move Demosaic Protocol Traits work. |
| [DHT Faithful Port Implementation Plan](_archive/plans/dht-port.md) | This archived plan documents the completed DHT Faithful Port work. |
| [DNG Compression Coverage and Corpus Status Plan](_archive/plans/dng-compression-coverage.md) | This archived plan documents the completed DNG Compression Coverage and Corpus Status work. |
| [DNG Output Color Management Implementation Plan](_archive/plans/dng-output-color.md) | This archived plan documents the completed DNG Output Color Management work. |
| [DNG Raw-IFD Compatibility and Performance Implementation Plan](_archive/plans/dng-raw-ifd-compatibility.md) | This archived plan documents the completed DNG Raw-IFD Compatibility and Performance work. |
| [DNG Roundtrip Finalization](_archive/plans/dng-roundtrip.md) | This archived plan documents the completed DNG Roundtrip Finalization work. |
| [Experimental DNG Zstandard and Writer Benchmarks](_archive/plans/dng-zstd-writer-benchmarks.md) | This archived plan documents the completed Experimental DNG Zstandard and Writer Benchmarks work. |
| [Remove `flashraw-codecs` and Own the Codec Paths](_archive/plans/flashraw-codec-removal.md) | This archived plan documents the completed Remove `flashraw-codecs` and Own the Codec Paths work. |
| [Independent Fornax CLI Implementation Plan](_archive/plans/fornax-cli.md) | This archived plan documents the completed Independent Fornax CLI work. |
| [Extract `fornax-codec` Implementation Plan](_archive/plans/fornax-codec-extraction.md) | This archived plan documents the completed Extract `fornax-codec` work. |
| [Initialize fornax-decoder Implementation Plan](_archive/plans/fornax-decoder-scaffold.md) | This archived plan documents the completed Initialize fornax-decoder work. |
| [Fujifilm RAF Implementation Plan](_archive/plans/fujifilm-raf-decoder.md) | This archived plan documents the completed Fujifilm RAF work. |
| [Cheap Golden Decode Hash CI Implementation Plan](_archive/plans/golden-corpus-ci.md) | This archived plan documents the completed Cheap Golden Decode Hash CI work. |
| [Hasselblad 3FR Decoder](_archive/plans/hasselblad-3fr-decoder.md) | This archived plan documents the completed Hasselblad 3FR decoder work. |
| [LMMSE faithful port](_archive/plans/lmmse-port.md) | This archived plan documents the completed LMMSE faithful port work. |
| [TIFF LZW, PackBits, and F64 Codec Support](_archive/plans/lzw-packbits-codecs.md) | This archived plan documents the completed TIFF LZW, PackBits, and F64 Codec Support work. |
| [Nikon NEF Huffman Lossless Implementation Plan](_archive/plans/nikon-nef-huffman.md) | This archived plan documents the completed Nikon NEF Huffman Lossless work. |
| [Nikon NEF Uncompressed and Packed Implementation Plan](_archive/plans/nikon-nef-packed.md) | This archived plan documents the completed Nikon NEF Uncompressed and Packed work. |
| [Panasonic RW2 Decoder](_archive/plans/panasonic-rw2-decoder.md) | This archived plan documents the completed Panasonic RW2 Decoder work. |
| [Phase One IIQ Implementation Plan](_archive/plans/phase-one-iiq-decoder.md) | This archived plan documents the completed Phase One IIQ work. |
| [RawTherapee demosaic reference integration](_archive/plans/rawtherapee-demosaic-reference.md) | This archived plan documents the completed RawTherapee demosaic reference integration work. |
| [Sony ARW/NRW Decoder](_archive/plans/sony-arw-decoder.md) | This archived plan documents the completed Sony ARW/NRW Decoder work. |
| [TIFF and DNG Crate Ownership Implementation Plan](_archive/plans/tiff-dng-crate-split.md) | This archived plan documents the completed TIFF and DNG Crate Ownership work. |
| [VNG faithful behavioral implementation](_archive/plans/vng-port.md) | This archived plan documents the completed VNG faithful behavioral implementation work. |
| [VNG4 implementation plan](_archive/plans/vng4.md) | This archived plan documents the completed VNG4 work. |

### Archived workspace-refactor records

| Document | Summary |
| --- | --- |
| [Workspace refactor baseline](_archive/workspace-refactor/baseline-findings.md) | This archived workspace-refactor record documents Workspace refactor baseline. |
| [Workspace refactor baseline](_archive/workspace-refactor/baseline-plan.md) | This archived workspace-refactor record documents Workspace refactor baseline. |
| [Codec Sample Buffer Step](_archive/workspace-refactor/codec-sample-buffer-step.md) | This archived workspace-refactor record documents Codec Sample Buffer Step. |
| [TIFF/DNG Compression ID Ownership Audit](_archive/workspace-refactor/compression-id-audit-implementation.md) | This archived workspace-refactor record documents TIFF/DNG Compression ID Ownership Audit. |
| [Compression ID Ownership Audit Implementation Plan](_archive/workspace-refactor/compression-id-audit-plan.md) | This archived workspace-refactor record documents Compression ID Ownership Audit Implementation Plan. |
| [Decode Baseline Step](_archive/workspace-refactor/decode-baseline-step.md) | This archived workspace-refactor record documents Decode Baseline Step. |
| [Move DNG-Owned Constants Implementation Plan](_archive/workspace-refactor/dng-constants-implementation.md) | This archived workspace-refactor record documents Move DNG-Owned Constants Implementation Plan. |
| [DNG Constants Step](_archive/workspace-refactor/dng-constants-step.md) | This archived workspace-refactor record documents DNG Constants Step. |
| [LZW and PackBits Codec Implementation Plan](_archive/workspace-refactor/lzw-packbits-plan.md) | This archived workspace-refactor record documents LZW and PackBits Codec Implementation Plan. |
| [Workspace refactor measurements](_archive/workspace-refactor/measurements.md) | This archived workspace-refactor record documents Workspace refactor measurements. |
| [Multi-band container options](_archive/workspace-refactor/multiband-container-options.md) | This archived workspace-refactor record documents Multi-band container options. |
| [Multi-band container options record](_archive/workspace-refactor/multiband-container-record.md) | This archived workspace-refactor record documents Multi-band container options record. |
| [Multi-Band Container Design Note Implementation Plan](_archive/workspace-refactor/multiband-design-plan.md) | This archived workspace-refactor record documents Multi-Band Container Design Note Implementation Plan. |
| [Optional TIFF Raster Implementation Plan](_archive/workspace-refactor/optional-tiff-raster-plan.md) | This archived workspace-refactor record documents Optional TIFF Raster Implementation Plan. |
| [Workspace refactor Phase A](_archive/workspace-refactor/phase-a.md) | This archived workspace-refactor record documents Workspace refactor Phase A. |
| [Workspace refactor Phase B-C handoff](_archive/workspace-refactor/phase-b-c-handoff.md) | This archived workspace-refactor record documents Workspace refactor Phase B-C handoff. |
| [Publish Readiness Implementation Plan](_archive/workspace-refactor/publish-readiness-implementation.md) | This archived workspace-refactor record documents Publish Readiness Implementation Plan. |
| [Publish Readiness Report Implementation Plan](_archive/workspace-refactor/publish-readiness-plan.md) | This archived workspace-refactor record documents Publish Readiness Report Implementation Plan. |
| [Publish Readiness Report](_archive/workspace-refactor/publish-readiness-report.md) | This archived workspace-refactor record documents Publish Readiness Report. |
| [Raw Pipeline Merge Step](_archive/workspace-refactor/raw-pipeline-merge-step.md) | This archived workspace-refactor record documents Raw Pipeline Merge Step. |
| [Merge Preprocessing and Postprocessing Implementation Plan](_archive/workspace-refactor/raw-pipeline-merge.md) | This archived workspace-refactor record documents Merge Preprocessing and Postprocessing Implementation Plan. |
| [Steps 1-6 workspace refactor review summary](_archive/workspace-refactor/steps-1-6-review.md) | This archived workspace-refactor record documents Steps 1-6 workspace refactor review summary. |
| [Generic TIFF Accessors Implementation Plan](_archive/workspace-refactor/tiff-accessors-implementation.md) | This archived workspace-refactor record documents Generic TIFF Accessors Implementation Plan. |
| [TIFF Accessors Step](_archive/workspace-refactor/tiff-accessors-step.md) | This archived workspace-refactor record documents TIFF Accessors Step. |
| [Optional TIFF Raster Feature Plan](_archive/workspace-refactor/tiff-raster-feature.md) | This archived workspace-refactor record documents Optional TIFF Raster Feature Plan. |
| [TIFF Raster Layout Support Implementation Plan](_archive/workspace-refactor/tiff-raster-implementation.md) | This archived workspace-refactor record documents TIFF Raster Layout Support Implementation Plan. |
| [TIFF Raster Step](_archive/workspace-refactor/tiff-raster-step.md) | This archived workspace-refactor record documents TIFF Raster Step. |
| [Workspace refactor step triggers](_archive/workspace-refactor/workspace-refactor-steps.md) | This archived workspace-refactor record documents Workspace refactor step triggers. |

### Archived design records

| Document | Summary |
| --- | --- |
| [DCB faithful behavioral port](_archive/designs/dcb-port-design.md) | This archived design note documents the selected DCB faithful behavioral port approach. |
| [DHT faithful behavioral port](_archive/designs/dht-port-design.md) | This archived design note documents the selected DHT faithful behavioral port approach. |

### Documentation conventions

| Document | Summary |
| --- | --- |
| [Documentation Conventions](CONVENTIONS.md) | This guide defines where documentation belongs and the naming and structure rules future contributions should follow. |
