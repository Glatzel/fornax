# FlashRaw reference notes

This reference note records the pinned FlashRaw benchmark adapter, required neutral settings, and implementation differences.

## Sources

- Crate `flashraw` 0.2.0:
  [crates.io](https://crates.io/crates/flashraw/0.2.0) and
  [API documentation](https://docs.rs/flashraw/0.2.0/flashraw/).
- Repository `https://github.com/ybouane/flashraw`, release and package version
  `0.2.0`.
- License: MIT OR Apache-2.0.

The exact package is pinned as a direct dev-dependency of `fornax-decoder`.
Rawler 0.8.0 is pinned beside it as the other benchmark-only reference.

## Benchmark role

FlashRaw is a reference decoder, not a production Fornax dependency. The adapter
exposes the three measurements shared by all benchmark implementations:
`decode`, `raw_mosaic`, and `cropped_mosaic`.

FlashRaw does not expose Fornax's normalized process metadata or a typed TIFF
`Directory` equivalent. The adapter therefore returns unsupported for
`process_metadata` and does not implement DNG metadata. Fornax's
`dng_metadata` and `process_metadata` timings remain diagnostic-only.

## Adapter behavior

`FlashRaw::open_path` performs container parsing. The adapter then calls
`FlashRaw::decode` with these benchmark-neutral settings:

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

The outer CI matrix also pins `RAYON_NUM_THREADS=1` and `OMP_NUM_THREADS=1` so
reference-library defaults do not turn the comparison into an all-core versus
single-thread measurement.

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
