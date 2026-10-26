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
- 🔒 **Safe abstractions** — Rust bindings to LibRaw with memory safety in mind.

## Components

- **fornax-core**: Foundational image aliases, Bayer types, and primitives.
- **fornax-decoder-dnc**: A wrapper of Adobe DNG Converter.
- **fornax-decoder-libraw**: Safe wrapper around the native LibRaw library.
- **fornax-decoder-protocol**: Shared decoder trait, errors, and decoded output types.
- **fornax**: Simple high-level raw processing manager.
- **fornax-dalim**: Rust native raw image post processor.
- **fornax-metadata**: Universal RAW metadata and level information.
- **fornax-py**: Python package for raw processing manager with Rust backend.
- **libraw-sys**: Libraw rust bindings.
