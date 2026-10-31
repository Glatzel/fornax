# fornax-demosaic-reference

Internal backend that exposes the pinned LibRaw and darktable implementations
through a small Rust API for Fornax's exact-reference demosaic variants.

The backend is intentionally `publish = false`. LibRaw is dynamically linked
under the terms in `LICENSE.LibRaw`; darktable source is downloaded by the
repository setup scripts and remains subject to darktable's GPL-3.0 license.

The crate converts a single-channel Bayer mosaic into interleaved 16-bit RGB.
It does not decode RAW files or perform preprocessing.
