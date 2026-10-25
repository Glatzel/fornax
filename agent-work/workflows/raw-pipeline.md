# Fornax RAW Pipeline Workflow

## Scope

Only conventional 2x2 RGB Bayer input is supported: RGGB, BGGR, GRBG, and
GBRG. X-Trans, Foveon, Quad Bayer, four-color, monochrome, and non-mosaic
payloads are rejected.

```text
LibRaw unpack
  -> DecodedRaw(raw f32 codes + nested Metadata)
  -> DecodedRaw::preprocess()
  -> (normalized full-raster RawImage + remaining metadata)
  -> RawImage::cropped(ActiveArea)
  -> parallel linear demosaic
  -> linear camera RGB
```

The decoder does not normalize, crop, white-balance, color-correct, or render
the image. Preprocessing performs black subtraction, white-level normalization,
and clipping. Cropping is a metadata-free image-rs view. Postprocessing is not
implemented; its metadata is preserved for future design.

## Data Rules

- `RawImage` contains only one full-raster f32 plane and no metadata.
- Decoded f32 values are raw sensor codes and may exceed one.
- `DecodedRaw` pairs `RawImage` with nested `Metadata`.
- Preprocess metadata includes ActiveArea, full-raster CFA order, and selected
  black/white levels.
- Preprocessing preserves full raster dimensions and clips once to `[0,1]`.
- Public uncropped image access is hidden; public consumers use `cropped`.
- Demosaic metadata contains the crop rectangle and Bayer order rebased to its
  origin.
- Postprocess metadata preserves white balance and color matrices without
  applying them.

## Metadata

Metadata uses named stage structs:

- `DecoderMetadata` with named vendor and capture groups;
- `PreprocessMetadata` with named CFA and level groups;
- `PostprocessMetadata` with named white-balance and color-matrix groups;
- `ProcessedMetadata` with decoder, demosaic, and postprocess groups.

Uncommon fields use one top-level `HashMap<String, MetadataValue>`. Use
namespaced keys such as `libraw.dng_version` or `libraw.raw_pitch`; do not
duplicate a field that has a typed home.

## Validation

Keep validation only at:

- `RawImage` construction;
- `DecodedRaw` construction;
- LibRaw layout and level mapping;
- unsafe LibRaw row/pointer access.

Do not add separate validators for invariants already guaranteed by those
boundaries. Do not add Rustdoc documentation, examples, or doctests in this
refactor.

The CR2 comparison requires exact raw decode, preprocessing within one 16-bit
code, and unsaturated demosaic max <= 8, mean <= 2, RMS <= 3.

## Task and PR Protocol

Each task has one TOML manifest. Work on `task/<id>-<slug>` in
`.worktrees/<id>-<slug>`, target `refactor/raw-pipeline-v2`, and open one PR.
GitHub review is the approval record. Only the user merges task PRs or promotes
the integration branch.
