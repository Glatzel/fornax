"""Shared geometry, tile metadata, and Bayer pattern definitions for compare-demosaic."""

from __future__ import annotations

from dataclasses import dataclass

CANVAS_WIDTH = 2048
CANVAS_HEIGHT = 2048
GRID_COLS = 4
GRID_ROWS = 4
TILE_SIZE = 512

HEADER_HEIGHT = 40
ROI_PAD_X = 32
ROI_PAD_TOP = 48
ROI_SIZE = 448
SUPERSAMPLE_FACTOR = 4
NOISE_SEED = 20261124

# 6 flat patches (3 cols x 2 rows) for `noisy_flat_fields`: (linear_luminance, gaussian_sigma)
NOISY_PATCH_SPECS: tuple[tuple[float, float], ...] = (
    (0.00, 0.020),
    (0.05, 0.015),
    (0.18, 0.030),
    (0.18, 0.060),
    (0.50, 0.040),
    (0.85, 0.050),
)

# Exact 2x2 Bayer masks matching `crates/fornax-raw/src/bayer.rs`:
# 0 = R, 1 = G, 2 = B. Indexed as mask[y & 1][x & 1].
BAYER_PATTERNS: dict[str, tuple[tuple[int, int], tuple[int, int]]] = {
    "rggb": ((0, 1), (1, 2)),
    "bggr": ((2, 1), (1, 0)),
    "grbg": ((1, 0), (2, 1)),
    "gbrg": ((1, 2), (0, 1)),
}


@dataclass(frozen=True)
class TileSpec:
    """Definition of one 512x512 tile and its even-aligned 448x448 evaluation ROI."""

    id: str
    row: int
    col: int
    title: str
    badge: str
    category: str  # "clean_srgb" | "noisy_mosaic" | "unclamped_float"
    value_range: tuple[float, float]
    achromatic: bool = False

    def tile_origin(self) -> tuple[int, int]:
        """Return (x0, y0) of the outer 512x512 tile."""
        return (self.col * TILE_SIZE, self.row * TILE_SIZE)

    def roi_bounds(self) -> tuple[int, int, int, int]:
        """Return (x0, y0, width, height) of the even-aligned 448x448 evaluation ROI."""
        tx, ty = self.tile_origin()
        return (tx + ROI_PAD_X, ty + ROI_PAD_TOP, ROI_SIZE, ROI_SIZE)

    def roi_slices(self) -> tuple[slice, slice]:
        """Return (y_slice, x_slice) for indexing (H, W, ...) numpy arrays."""
        x0, y0, w, h = self.roi_bounds()
        return (slice(y0, y0 + h), slice(x0, x0 + w))


TILES: tuple[TileSpec, ...] = (
    # Row 0: Spatial Resolution (clean [0, 1])
    TileSpec(
        id="siemens_star",
        row=0,
        col=0,
        title="Siemens Star (72-spoke)",
        badge="[0, 1] · Resolution",
        category="clean_srgb",
        value_range=(0.0, 1.0),
        achromatic=True,
    ),
    TileSpec(
        id="zone_plate",
        row=0,
        col=1,
        title="Zone Plate (DC-Nyquist)",
        badge="[0, 1] · Analytic f32",
        category="clean_srgb",
        value_range=(0.0, 1.0),
        achromatic=True,
    ),
    TileSpec(
        id="line_pairs_edges",
        row=0,
        col=2,
        title="Line Pairs & 5 deg Edges",
        badge="[0, 1] · MTF / Bars",
        category="clean_srgb",
        value_range=(0.0, 1.0),
        achromatic=False,
    ),
    TileSpec(
        id="curved_edges",
        row=0,
        col=3,
        title="Curved Edges & Fine Arcs",
        badge="[0, 1] · Curves",
        category="clean_srgb",
        value_range=(0.0, 1.0),
        achromatic=False,
    ),
    # Row 1: Sampling & Aliasing (clean [0, 1])
    TileSpec(
        id="bw_checkerboard",
        row=1,
        col=0,
        title="B/W Checkerboards (1-4 px)",
        badge="[0, 1] · Aliasing",
        category="clean_srgb",
        value_range=(0.0, 1.0),
        achromatic=True,
    ),
    TileSpec(
        id="woven_texture",
        row=1,
        col=1,
        title="Fine Woven Texture",
        badge="[0, 1] · Micro-Weave",
        category="clean_srgb",
        value_range=(0.0, 1.0),
        achromatic=False,
    ),
    TileSpec(
        id="freq_chirps",
        row=1,
        col=2,
        title="1D Frequency Chirps",
        badge="[0, 1] · Analytic f32",
        category="clean_srgb",
        value_range=(0.0, 1.0),
        achromatic=True,
    ),
    TileSpec(
        id="impulses_grid",
        row=1,
        col=3,
        title="Phase Impulses & Micro-Grid",
        badge="[0, 1] · 4 Bayer Phases",
        category="clean_srgb",
        value_range=(0.0, 1.0),
        achromatic=False,
    ),
    # Row 2: Chromatic & Tonal (clean [0, 1])
    TileSpec(
        id="chroma_checkerboard",
        row=2,
        col=0,
        title="R/B & G/M Checkerboards",
        badge="[0, 1] · Chroma 1-2 px",
        category="clean_srgb",
        value_range=(0.0, 1.0),
        achromatic=False,
    ),
    TileSpec(
        id="color_boundaries",
        row=2,
        col=1,
        title="Saturated Color Boundaries",
        badge="[0, 1] · Color Star",
        category="clean_srgb",
        value_range=(0.0, 1.0),
        achromatic=False,
    ),
    TileSpec(
        id="gradients_patches",
        row=2,
        col=2,
        title="Grayscale, RGB/CMY & Patches",
        badge="[0, 1] · Tonal",
        category="clean_srgb",
        value_range=(0.0, 1.0),
        achromatic=False,
    ),
    TileSpec(
        id="dead_leaves",
        row=2,
        col=3,
        title="Random Geometric (Dead Leaves)",
        badge="[0, 1] · ISO 19567",
        category="clean_srgb",
        value_range=(0.0, 1.0),
        achromatic=False,
    ),
    # Row 3: Noise & Unclamped Float
    TileSpec(
        id="noisy_flat_fields",
        row=3,
        col=0,
        title="Noisy Flat Fields",
        badge="Mosaic Noise · Signed Dark",
        category="noisy_mosaic",
        value_range=(0.0, 1.0),
        achromatic=True,
    ),
    TileSpec(
        id="signed_zero_crossing",
        row=3,
        col=1,
        title="Signed Zero-Crossing",
        badge="[-0.25, +0.25] · Analytic f32",
        category="unclamped_float",
        value_range=(-0.25, 0.25),
        achromatic=True,
    ),
    TileSpec(
        id="hdr_specular",
        row=3,
        col=2,
        title="HDR Specular Highlights",
        badge="[0.0, 4.0] · Analytic f32",
        category="unclamped_float",
        value_range=(0.0, 4.0),
        achromatic=False,
    ),
    TileSpec(
        id="wide_range_signed_hdr",
        row=3,
        col=3,
        title="Wide-Range Signed-to-HDR",
        badge="[-0.5, +2.5] · Analytic f32",
        category="unclamped_float",
        value_range=(-0.5, 2.5),
        achromatic=False,
    ),
)
