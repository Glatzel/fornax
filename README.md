# Fornax

![Release](https://img.shields.io/github/v/release/Glatzel/fornax)
![CI](https://github.com/Glatzel/fornax/actions/workflows/ci.yml/badge.svg?branch=main)
[![codecov](https://codecov.io/gh/Glatzel/fornax/graph/badge.svg?token=GrOFsrR2x7)](https://codecov.io/gh/Glatzel/fornax)
[![CodSpeed Badge](https://img.shields.io/endpoint?url=https://app.codspeed.io//badge.json)](https://app.codspeed.io//Glatzel/fornax?utm_source=badge)

**Fornax** is an extensive raw image processing library.
Its goal is to provide a one-stop solution for raw image processing with both simple high-level APIs and low-level control, while also enabling users to extend or plug in their own raw image processing algorithms.

Fornax uses built-in or custom decoders and post-processors to produce `ImageBuffer`s (from the [`image`](https://crates.io/crates/image) crate) that can be further transformed into other data types for analysis or display.

## Key Features

- 🖼 **Flexible decoding pipeline** — Choose built-in or custom decoders.
- ⚙️ **Extensible post-processing** — Plug in your own algorithms.
- 🦀 **Rust + Python support** — Native performance with Python accessibility.
- 🔒 **Pure-Rust decoding** — No C/C++ decoder dependency in the production
  image pipeline.

## Architecture

```mermaid
flowchart TB
  subgraph tools_group["Tools"]
    cli["fornax-cli<br/>Batch RAW converter"]
    tracker["decoder-tracker<br/>RAW decoder regression tracking"]
    compare["compare-demosaic<br/>Demosaic comparison harness"]
  end

  subgraph facade_group["Facade"]
    raw["fornax-raw<br/>High-level RAW pipeline façade"]
  end

  subgraph workflow_group["Raw workflow"]
    decoder["fornax-decoder<br/>Thin default decoder facade"]
    pipeline["fornax-raw-pipeline<br/>Preprocessing, demosaicing, and color processing"]
  end

  subgraph formats_group["Formats"]
    dng["fornax-dng<br/>DNG parsing, transforms, and writing"]
    arw["fornax-arw<br/>Sony ARW/NRW decoding"]
  end

  subgraph neutral_group["Format-neutral"]
    tiff["fornax-tiff<br/>Generic TIFF parsing and writing"]
    codec["fornax-codec<br/>Format-neutral raw sample codecs"]
  end

  subgraph core_group["Core"]
    raw_core["fornax-raw-core<br/>Shared pipeline types and stage traits"]
  end

  subgraph dev_group["Dev support"]
    devtool["fornax-devtool<br/>Deterministic fixtures and benchmark helpers"]
  end

  cli --> dng
  cli --> raw
  tracker --> decoder
  tracker --> dng
  tracker --> tiff
  compare --> decoder
  compare --> devtool
  compare --> pipeline

  raw --> decoder
  raw --> pipeline

  decoder --> arw
  decoder --> dng

  arw --> dng
  arw --> tiff
  dng --> codec
  dng --> tiff

  tiff --> codec

  classDef toolsClass fill:#fde68a,stroke:#b45309,stroke-width:1.5px,color:#111827;
  classDef facadeClass fill:#fbcfe8,stroke:#be185d,stroke-width:1.5px,color:#111827;
  classDef workflowClass fill:#bfdbfe,stroke:#1d4ed8,stroke-width:1.5px,color:#111827;
  classDef formatsClass fill:#bbf7d0,stroke:#15803d,stroke-width:1.5px,color:#111827;
  classDef neutralClass fill:#ddd6fe,stroke:#6d28d9,stroke-width:1.5px,color:#111827;
  classDef coreClass fill:#e5e7eb,stroke:#374151,stroke-width:1.5px,color:#111827;
  classDef devClass fill:#f3f4f6,stroke:#6b7280,stroke-width:1.5px,stroke-dasharray: 4 3,color:#111827;

  class cli,tracker,compare toolsClass;
  class raw facadeClass;
  class decoder,pipeline workflowClass;
  class dng,arw formatsClass;
  class tiff,codec neutralClass;
  class raw_core coreClass;
  class devtool devClass;

  style tools_group fill:#fffbeb,stroke:#b45309,stroke-width:1.5px,color:#92400e;
  style facade_group fill:#fdf2f8,stroke:#be185d,stroke-width:1.5px,color:#9d174d;
  style workflow_group fill:#eff6ff,stroke:#1d4ed8,stroke-width:1.5px,color:#1e40af;
  style formats_group fill:#f0fdf4,stroke:#15803d,stroke-width:1.5px,color:#166534;
  style neutral_group fill:#f5f3ff,stroke:#6d28d9,stroke-width:1.5px,color:#5b21b6;
  style core_group fill:#f9fafb,stroke:#374151,stroke-width:1.5px,color:#1f2937;
  style dev_group fill:#f9fafb,stroke:#6b7280,stroke-width:1.5px,color:#374151;
  linkStyle default stroke:#6b7280,stroke-width:1.5px;
```

Arrows point from a crate to a crate it depends on.
Every crate except fornax-cli, fornax-codec, fornax-tiff, and fornax-raw-core itself also depends on fornax-raw-core; those edges are omitted.

| Color | Group | Meaning |
| --- | --- | --- |
| Yellow | Tools | User-facing entry points and validation tools. |
| Pink | Facade | High-level API surface. |
| Blue | Raw workflow | Decode and post-decode processing stages. |
| Green | Formats | DNG- and Sony-specific handling. |
| Violet | Format-neutral | Reusable TIFF and sample-codec foundations. |
| Gray | Core | Shared types, metadata, errors, and stage contracts. |
| Light gray | Dev support | Fixtures, paths, and benchmark configuration. |

| Crate | Group | Role |
| --- | --- | --- |
| `fornax-cli` | Tools | Batch RAW converter. |
| `decoder-tracker` | Tools | Manual RAW decoder regression tracking. |
| `compare-demosaic` | Tools | Demosaic comparison harness. |
| `fornax-raw` | Facade | High-level façade for the RAW pipeline. |
| `fornax-decoder` | Raw workflow | Thin default decoder facade. |
| `fornax-raw-pipeline` | Raw workflow | Post-decode preprocessing, demosaicing, and color processing. |
| `fornax-dng` | Formats | Pure-Rust DNG parsing, transforms, and writing. |
| `fornax-arw` | Formats | Pure-Rust Sony ARW/NRW decoding. |
| `fornax-tiff` | Format-neutral | Generic TIFF parsing, values, and writing. |
| `fornax-codec` | Format-neutral | Format-neutral raw sample codecs. |
| `fornax-raw-core` | Core | Shared RAW pipeline types and stage traits. |
| `fornax-devtool` | Dev support | Deterministic fixtures and benchmark helpers. |

## Development

Install the development environment and pinned reference sources:

```bash
source scripts/setup.sh
```

On Windows, run the PowerShell setup from the repository root:

```powershell
./scripts/setup.ps1
```
