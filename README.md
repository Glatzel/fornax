# Fornax

![Release](https://img.shields.io/github/v/release/Glatzel/fornax)
![CI](https://github.com/Glatzel/fornax/actions/workflows/ci.yml/badge.svg?branch=main)
[![codecov](https://codecov.io/gh/Glatzel/fornax/graph/badge.svg?token=GrOFsrR2x7)](https://codecov.io/gh/Glatzel/fornax)

**Fornax** is an extensive raw image processing library.
Its goal is to provide a one-stop solution for raw image processing with both simple high-level APIs and low-level control, while also enabling users to extend or plug in their own raw image processing algorithms.

Fornax uses built-in or custom decoders and post-processors to produce `ImageBuffer`s (from the [`image`](https://crates.io/crates/image) crate) that can be further transformed into other data types for analysis or display.

## Key Features

- 🖼 **Flexible decoding pipeline** — Choose built-in or custom decoders.
- ⚙️ **Extensible post-processing** — Plug in your own algorithms.
- 🦀 **Rust + Python support** — Native performance with Python accessibility.
- 🔒 **Safe abstractions** — Rust bindings to LibRaw with memory safety in mind.

## Components

- **dnc**: A wrapper of Adobe dng converter.
- **fornax**: Simple high-level raw processing manager.
- **fornax-core**: Foundational traits and structs for decoder and post-processor.
- **fornax-dalim**: Rust native raw image post processor.
- **fornax-py**: Python package for raw processing manager with Rust backend.
- **libraw**: Safe wrapper around the native libraw library.
- **libraw-sys**: Libraw rust bindings.
