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
- 📷 **Native RAW formats** — DNG, Sony ARW/NRW, Nikon NEF, Fujifilm RAF, and
  single-shot Panasonic RW2 V4/V5/V6/V8.

## Components

- **fornax-raw-core**: Shared pipeline types, errors, and decoder/processor
  contracts.
- **fornax-bmff**: Bounded generic ISO-BMFF box and sample-table parsing.
- **fornax-cr3**: Canon CR3/CRM parsing and the audited lossless CRX handoff.
- **fornax-raw-pipeline**: Black/white-level normalization, demosaic
  algorithms, RGB reconstruction, and color processing.
- **fornax-raw**: High-level facade tying the stages together.
- **fornax-devtool**: Pinned fixtures and development helpers.
- **compare-demosaic**: Reference-versus-Fornax demosaic comparison harness.
- **fornax-py**: Python binding source, currently excluded from the Rust
  workspace while its public API is stabilized.

## Development

Install the development environment and pinned reference sources:

```bash
source scripts/setup.sh
```

On Windows, run the PowerShell setup from the repository root:

```powershell
./scripts/setup.ps1
```
