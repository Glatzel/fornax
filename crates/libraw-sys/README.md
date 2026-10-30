# libraw-sys

Low-level, `no_std` Rust FFI bindings to the LibRaw C API. Prefer the safe
`libraw` crate unless direct C API access is required.

```bash
LIBRAW_ROOT=/path/to/libraw cargo check -p libraw-sys
```

Most functions in this crate are unsafe. Applications must uphold LibRaw's
pointer, buffer, and lifetime requirements.
