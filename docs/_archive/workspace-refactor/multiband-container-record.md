# Multi-band container options record

## Goal

Create a documentation-only decision-support note that records the current
single-band sample path and compares additive `SampleBuffer` variants with a
separate multi-band/raster container, without selecting a container design.

## Scope

In scope:

- Document current `SampleBuffer` behavior for U8, U16, U32, F32, and F64,
  including the public `image::ImageBuffer<Luma<T>, Vec<T>>` exposure.
- Compare the two container options across source compatibility, exhaustive
  matching, dimensions and raw access, codec requests and predictor geometry,
  TIFF strip/tile layout, DNG adaptation and unsupported multi-channel
  `LinearRaw`, raw-workflow separation, `image` 0.x compatibility, and i16/f64
  storage consequences.
- Include a comparison table, unresolved questions, and a closing options and
  consequences section.
- Link the note from `docs/plans/workspace-refactor/README.md`.

Out of scope:

- Production code, tests, Cargo features, dependencies, versions, crate names,
  golden files, the Python API, or `TODO.md`.
- Defining or selecting a container shape, band count, variant names, i16/f64
  representation, public signatures, lifetimes, or bounds.
- Placing TIFF or raw-workflow types in `fornax-codec`.
- Compiling `compare-demosaic`, running lefthook, running benchmarks, or
  merging the pull request.

## Task checklist

- [x] Record the current single-band contracts from `SampleBuffer`, codec
  requests and predictors, TIFF raster layout, and the DNG adapter.
- [x] Analyze additive variants without naming or shaping the added variants.
- [x] Analyze a separate multi-band/raster container without defining its
  type, API, ownership, or placement.
- [x] Add the required comparison table, unresolved questions, and final
  options and consequences.
- [x] Link the evidence note from the workspace-refactor index.
- [x] Run documentation, dependency-tree, golden-hash, and diff validation.
- [x] Commit and open one documentation PR, then stop.

## Files expected to change

- Create: `docs/plans/chore-multiband-note.md`
- Create: `docs/plans/workspace-refactor/multiband-note.md`
- Modify: `docs/plans/workspace-refactor/README.md`

## Validation

Run from the worktree root:

```bash
scripts/agent-validation.sh --docs-only
git diff --check
cargo tree -p fornax-codec -e normal --depth 1
cargo tree -p fornax-tiff -e normal --depth 1
cargo run -q -p decoder-tracker -- hash \
  --manifest crates/decoder-tracker/golden/selected-manifest.csv \
  --check crates/decoder-tracker/golden/decode-hashes.csv
```

Do not run local Markdown lint, compile `compare-demosaic`, run lefthook, or
run benchmarks. Local LFS materialization is not a blocker; CI is authoritative.

## Decisions and notes

- This is an evidence and consequence record, not an implementation design.
- The current mono path is evidence, not a proposed future architecture.
- The additive and separate-container sections must remain alternatives.
- The note ends with options and consequences, not a recommendation.
- No new production decision is delegated to the implementing agent.
- `scripts/agent-validation.sh --docs-only` passed.
- `git diff --check` passed.
- Both required dependency trees were recorded and passed.
- The golden hash check could not materialize its selected local LFS object.
  CI is authoritative for this documentation-only change.
