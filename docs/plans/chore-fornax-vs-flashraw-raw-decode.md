# Fornax versus FlashRaw raw decode

## Goal

Measure native Fornax and FlashRaw 0.3.1 from file path to in-memory raw
mosaic, verify the raw outputs, and identify concrete Fornax performance gaps
without changing decoder behavior.

## Scope

In scope:

- FlashRaw: `open_path` plus strict raw `decode`, with normalized file open and
  decode sub-timings when separable.
- Fornax: `fornax_raw_decoder::Decoder::decode` plus `raw_mosaic`, using the
  native TIFF/DNG, codec, and vendor paths.
- The 14 DNG, ARW, CR3, and NEF samples in the existing lossless/uncompressed
  manifest.
- Raw output hashing outside timed regions, mismatch diagnostics, unsupported
  result rows, three process batches, and one- and sixteen-thread runs.
- Profiling and source comparison only where Fornax is materially slower.
- Ranked improvement proposals; no optimization implementation.

Out of scope:

- Crop, cast, preprocessing, hashing, or CSV writes inside timing.
- LibRaw, DNC, or Rawler as performance competitors. Rawler/LibRaw may only be
  used to investigate a correctness mismatch.
- Lossy modes, decoder behavior changes, and optimization patches.

## Task checklist

- [x] Replace the old multi-stage probe with one raw-decode stage and a
  `fornax`/`flashraw` selector.
- [x] Record decoder, backend, threads, source metadata, timings, faults, raw
  hash, and status; continue past unsupported or mismatching outputs.
- [x] Run three independent batches for each decoder at one and sixteen
  threads with two warmups and five rounds.
- [x] Aggregate medians and batch spreads, excluding unsupported and
  mismatching samples from performance comparisons.
- [x] Investigate every mismatch and identify the likely incorrect decoder.
- [x] Profile clearly slower Fornax paths and compare source-level hot loops
  with FlashRaw.
- [x] Add a dated results CSV and a short report under decoder performance
  documentation.
- [x] Commit the probe separately from the report and data.
- [x] Run focused tests, formatting, clippy, link checks, and final review.

## Files expected to change

- `crates/fornax-raw-decoder/benches/flashraw_031_probe.rs`
- `docs/reference/decoders/perf/data/<dated-results>.csv`
- `docs/reference/decoders/perf/<dated-report>.md`
- `docs/plans/chore-fornax-vs-flashraw-raw-decode.md`

## Validation

- probe compilation and focused raw decoder tests
- twelve independent measurement batches
- raw hash comparison for every supported decoder/sample pair
- repeated noisy measurements rather than treating a single result as stable
- nightly formatting, focused clippy, documentation link check, and
  `git diff --check`

## Decisions and notes

- FlashRaw's raw stage includes `open_path` and `decode`; its separate
  `decode_ns` column starts after `open_path` returns.
- Fornax currently parses in `Decoder::decode` and materializes samples in
  `raw_mosaic`, so both calls are inside its timed region. Its `decode_ns` is
  therefore equivalent to total time rather than a like-for-like FlashRaw
  sub-stage.
- Unsupported and raw-mismatch samples remain in the CSV with status fields but
  are excluded from performance summaries.
- RAW payloads are hard-linked from the main checkout and treated as read-only.
- Fornax supported all 14 samples. Two had incorrect Fornax output (Sony
  ILCE-7RM4 uncompressed and Pentax K-1 LJPEG); FlashRaw matched Rawler on both.
- Leica SL2 was hash-correct but excluded from performance comparison because
  Fornax applies `OpcodeList1` while the FlashRaw probe disables opcodes.
