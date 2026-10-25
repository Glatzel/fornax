# Fornax

![Release](https://img.shields.io/github/v/release/Glatzel/fornax)
![CI](https://github.com/Glatzel/fornax/actions/workflows/ci.yml/badge.svg?branch=main)
[![codecov](https://codecov.io/gh/Glatzel/fornax/graph/badge.svg?token=GrOFsrR2x7)](https://codecov.io/gh/Glatzel/fornax)
[![CodSpeed Badge](https://img.shields.io/endpoint?url=https://app.codspeed.io//badge.json)](https://app.codspeed.io//Glatzel/fornax?utm_source=badge)

Fornax is a Rust RAW processing library for conventional 2x2 RGB Bayer images.
The current path decodes raw sensor codes through LibRaw, normalizes them in a
separate metadata-driven preprocessing step, exposes a zero-copy ActiveArea
view, and performs parallel bilinear demosaicing.

White balance, camera-to-sRGB correction, gamma, and output rendering are not
applied. LibRaw as-shot white balance and `rgb_cam` are preserved for a future,
not-yet-designed postprocess pipeline.

## Workspace

- **fornax**: normalized decoding model, LibRaw adapter, and Bayer demosaicing.
- **fornax-devtool**: fixture and example output helpers.
- **fornax-py**: Python bindings under migration.
- **dnc**: Adobe DNG Converter wrapper.
- **libraw-sys**: generated LibRaw bindings used by the private adapter.

## Development

Run `pixi install`, initialize `external/raw-images`, then configure
`LIBRAW_ROOT` as described in `scripts/setup.ps1`. The primary checks are:

```bash
cargo test -p fornax --lib
cargo test -p fornax --features backend-libraw --lib
cargo test -p fornax --features backend-libraw --examples
python3 agent-work/tasks/validate.py
```
