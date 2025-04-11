# Fornax

![Release](https://img.shields.io/github/v/release/Glatzel/fornax)
![CI](https://github.com/Glatzel/fornax/actions/workflows/ci.yml/badge.svg?branch=main)
[![codecov](https://codecov.io/gh/Glatzel/fornax/graph/badge.svg?token=GrOFsrR2x7)](https://codecov.io/gh/Glatzel/fornax)

Fornax is a extensive raw image processing library. The goal of the library is to provide a one-stop solusion for raw image image processing with both simple high-level apis and low level controls, and also allow users to extend their own raw image processing algorithms.

It takes built-in or custom decoder and post-processor to generate `ImageBuffer` of `image` crate that can be used convert to other data type for further processing.

## Components

- **dnc**: A wrapper of Adobe dng converter.
- **fornax**: Simple high-level raw processing manager.
- **fornax-core**: Foundational traits and structs for decoder and post-processor.
- **fornax-py**: Python package for raw processing manager with Rust backend.
- **libraw**: Safe wrapper around the native libraw library.
- **libraw-sys**: Libraw rust bindings.
