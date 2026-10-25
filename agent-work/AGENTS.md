# Fornax Agent Instructions

The RAW pipeline supports conventional 2x2 RGB Bayer only. Decoding copies raw
sensor codes into an f32 `RawImage`; preprocessing normalizes the full raster;
cropping is a metadata-free image-rs view; and demosaicing produces linear
camera RGB. White balance and color correction metadata are preserved in the
postprocess metadata group but are not applied.

## Current boundaries

- LibRaw decoding owns backend access, raw sample copying, CFA validation,
  ActiveArea discovery, and metadata copying.
- `RawImage` owns one full-raster `F32Plane` and no metadata.
- `DecodedRaw` pairs `RawImage` with nested `Metadata` and can split them
  without copying.
- `DecodedRaw::preprocess` consumes the preprocess metadata group and returns
  normalized `RawImage` plus the metadata needed by demosaic/postprocess.
- `RawImage::cropped` returns the ActiveArea view used by public callers.
- `RawImage::full_image` is hidden and exists only for development tooling.
- `demosaic` accepts an image-rs view plus `BayerPattern` and always uses Rayon.
- Nested `PostprocessMetadata` preserves white balance and color matrices
  unchanged.
- A future postprocess may implement a dcraw-like linear workflow. Its design
  and color-transform behavior are intentionally undecided; do not add one
  implicitly.

## API style

Follow the concrete function style used on `main`:

- prefer small single-purpose functions and methods;
- use `const fn new` and one-line `const fn` getters where possible;
- use direct parameters rather than option, registry, or strategy wrappers;
- return `Result` at decoder/FFI and validated construction boundaries;
- keep infallible image operations infallible; and
- do not add Rustdoc examples or doctests for this refactor.

Do not reintroduce `SampleSpec`, `StoredPlane`, `RawImage`, metadata provenance
trees, CFA shape/color abstractions, `DemosaicAlgorithm`, `RawSnapshot`, or
duplicate validation helpers unless a new approved task requires them.

## Task workflow

1. Read the active manifest under `agent-work/tasks/raw-pipeline/`.
2. Fetch dependencies and create one branch/worktree from
   `origin/refactor/raw-pipeline-v2`:

   ```bash
   git worktree add -b task/<id>-<slug> .worktrees/<id>-<slug> \
     origin/refactor/raw-pipeline-v2
   ```

3. Implement only the manifest scope and update its acceptance evidence.
4. Run the manifest validation commands and record actual results.
5. Commit, push, and open one PR targeting `refactor/raw-pipeline-v2` with a
   semantic title such as `refactor(impl-core-002): simplify bayer pipeline`.
6. Mark the task `Done` in `tasks/STATUS.md`, report the PR URL, and stop.
   Never merge or close the task PR.

The user initiates further changes or merging. Keep one active task unless a
new manifest intentionally defines a dependency.

## Validation

```bash
python3 agent-work/tasks/validate.py
cargo test -p fornax --lib
cargo test -p fornax --features backend-libraw --lib
cargo test -p fornax --features backend-libraw --examples
cargo fmt --check
cargo clippy -p fornax --all-targets --features backend-libraw -- -D warnings
```

LibRaw tests require `LIBRAW_ROOT` and the platform library path configured by
`scripts/setup.ps1`, plus the `external/raw-images` submodule.
