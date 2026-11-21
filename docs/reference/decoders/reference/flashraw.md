# FlashRaw reference notes

This note records the pinned FlashRaw version, benchmark adapter settings,
version-relevant behavior, and known differences from Fornax.

## Sources

- Crate `flashraw` 0.4.0:
  [crates.io](https://crates.io/crates/flashraw/0.4.0) and
  [API documentation](https://docs.rs/flashraw/0.4.0/flashraw/).
- Release tag
  [`v0.4.0`](https://github.com/ybouane/flashraw/releases/tag/v0.4.0),
  package and release commit
  `c6ee7c28ee5e3e34c07502de4ad11fcff66551d1`.
- License: MIT OR Apache-2.0.

The exact package is pinned as a direct dev-dependency of
`fornax-raw-decoder`. Rawler 0.8.0 is pinned beside it as the other
benchmark-only reference.

## Version-relevant changes

FlashRaw 0.4.0 preserves the Fornax-used `FlashRaw`, `DecodeOptions`,
`RawFrame`, and `RawPixelData` surface. Its release change is cooperative
cancellation: decoders and processing stages check a `CancelToken` between
bounded work units and return `Error::Cancelled` without a partial frame. With
no cancellation requested, the upstream release note says output remains
bit-identical and timing changes are within measurement noise.

The 0.3.x comparison remains useful historical context: versus 0.2.0 it added
64-bit bit pumps, lookup-based Huffman decoding, SIMD packed unpacking,
tile/strip/stripe parallelism, zeroed output allocation, speculative parallel
entropy decoding, and parallel LJPEG byte unstuffing. Raw output stayed bit
identical, Sony LJPEG improved about 30%, Nikon Huffman improved 27-29%, and DNG
LJPEG regressed about 12%. The current 0.4.0 golden performance-set comparison
and reproduction data are in
[FlashRaw versus Fornax raw decode](../perf/flashraw-vs-fornax.md).

## Benchmark role

FlashRaw is a reference decoder, not a production Fornax dependency. The
general decoder adapter exposes `decode`, `raw_mosaic`, and `cropped_mosaic`.
The current cross-decoder probe measures only raw decode.

FlashRaw does not expose Fornax's normalized process metadata or a typed TIFF
`Directory` equivalent. The adapter therefore returns unsupported for
`process_metadata` and does not implement DNG metadata. Fornax's
`dng_metadata` and `process_metadata` timings remain diagnostic-only.

## Adapter behavior

`FlashRaw::open_path` performs container parsing. The general adapter then
calls `FlashRaw::decode` with these benchmark-neutral settings:

- CPU backend;
- one worker thread;
- `CorruptionPolicy::Strict`;
- `apply_opcodes = false`;
- `crop = false`;
- `OutputFormat::RawU16` for `SampleFormat::U16`;
- `OutputFormat::RawF32` for `SampleFormat::F32`.

The adapter validates a one-sample conventional 2x2 RGB Bayer frame before
accepting the result. `raw_mosaic` clones the storage-native samples.
`cropped_mosaic` converts to the requested benchmark primitive, crops to
FlashRaw's active area, and shrinks the backing allocation to the cropped size.

The raw comparison uses the same strict/raw settings with
`FORNAX_FLASHRAW_BACKEND=cpu|simd|auto`, explicit worker counts, and
`RAYON_NUM_THREADS` aligned to the Fornax run. Scalar `cpu` remains the
headline baseline; `auto` is available for fastest-option checks.

## Relationship to the removed codec dependency

An earlier migration removed `flashraw-codecs` from Fornax after independently
implementing the codec behavior needed by production decoders. The `flashraw`
decoder crate added here is a benchmark oracle only. It does not restore that
codec dependency or enter the normal dependency graph of any production crate.

## Known differences

- FlashRaw's default API uses all available worker threads and applies DNG
  opcodes; the benchmark explicitly disables both behaviors.
- Raw outputs retain sensor orientation and are not implicitly cropped.
- FlashRaw normalizes metadata into its own public model rather than preserving
  the source TIFF `Directory`.
- FlashRaw can decode formats beyond Fornax's supported decoder set, but the
  benchmark only selects known-passing conventional Bayer rows from the Fornax
  golden manifest.
