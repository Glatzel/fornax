# Clean decoder performance documentation

## Goal

Make `docs/reference/decoders` navigable without losing durable conclusions,
negative results, or reproduction evidence, and make decoder performance use
the generated golden corpus instead of a hand-picked manifest.

## Scope

In scope:

- keep one current cross-decoder performance report;
- keep one canonical golden-corpus result CSV and one derived metadata matrix;
- merge durable ARW and allocator conclusions into their proper active notes;
- archive superseded reports, raw profile packs, and crop/cast-era data;
- update indexes and every inbound link;
- make the raw decode probe default to the golden selected manifest and the
  golden metadata matrix.

Out of scope:

- production decoder changes;
- deleting historical evidence;
- changing the generated golden corpus or its hashes.

## Task checklist

- [x] Inventory active versus superseded decoder performance documents.
- [x] Fold FlashRaw 0.3.1 pin/API changes into the FlashRaw reference.
- [x] Fold ARW support gaps and the four-component LJPEG proposal into the
  Sony reference and canonical performance report.
- [x] Fold large-page allocation invariants into a current design note.
- [x] Replace the hand-picked measurement list with the golden selected
  manifest and a matrix derived from golden metadata.
- [x] Merge the separate baseline and uncompressed result CSVs into one
  canonical golden-corpus CSV.
- [x] Archive historical reports, experiment logs, and raw evidence.
- [x] Rewrite active and archive indexes.
- [x] Run the documentation link checker.
- [x] Commit the docs/probe cleanup on the existing PR branch.

## Files expected to change

- `docs/reference/decoders/{README.md,perf,design,reference}`
- `docs/archive/reference/decoders/perf`
- `docs/archive/README.md`
- `docs/README.md`
- `docs/decoder/benchmark.md`
- `crates/fornax-raw-decoder/benches/flashraw_031_probe.rs`

## Validation

- `python3 scripts/check-doc-links.py`
- focused probe compilation
- `git diff --check`

## Decisions and notes

- The golden selected manifest is generated and represents one
  lowest-pixel known-lossless row per decoder path; it is not a hand-curated
  brand/resolution matrix.
- No historical file is deleted. Superseded material moves to
  `docs/archive/reference/decoders/perf/`.
- The old crop/cast and pre-optimization reports remain archived because they
  document negative results and earlier measured states.
