# Linear demosaicing readiness

Fornax's `Demosaic::Linear` implementation is the conventional bilinear Bayer
reconstruction used as the baseline demosaicer.

## Classification

**Done: production ready.**

An independent review found no production correctness issue in the four Bayer
phase mappings, border interpolation, interior reconstruction, or row
chunking. The implementation operates directly on scene-linear samples,
preserves measured samples, and uses only finite averages with constants as
denominators.

## Reconstruction

For an interior red or blue center, missing green is the average of the four
cardinal green neighbors. The missing opposite primary is the average of the
four diagonal neighbors.

At a green center, the red and blue samples lie in perpendicular directions.
The missing red (or blue) channel is therefore the average of the two
same-color neighbors along its row or column. `GRedRow` and `GBlueRow` select
the horizontal and vertical assignments without treating the two green phases
as separate output channels.

At finite-image borders, each reconstruction averages only in-bounds neighbors
of the requested color and divides by their available count. This keeps
constant scenes neutral and preserves signed values without a global clamp.

## Current Fornax behavior

[`LinearLazy`](../../../crates/fornax-raw-pipeline/src/demosaic/linear.rs)
uses a checked border path and a compile-time channel-specialized interior
path. It produces normalized scene-linear RGB for all four conventional 2x2
Bayer patterns supported by `BayerPattern`.

Fixed 3x3 RGGB, BGGR, GRBG, and GBRG outputs are recorded as three-decimal
`insta` snapshots on Linux x86_64. The shared demosaic invariant tests also
cover uniform scenes, measured-sample preservation, signed values, and
independent row filling.

## Required follow-up

None.
