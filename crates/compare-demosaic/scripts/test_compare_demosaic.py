"""Tests for chart_spec, generate_chart, and evaluate."""

from __future__ import annotations

import math
from typing import TYPE_CHECKING

import numpy as np
import tifffile
from chart_spec import BAYER_PATTERNS, CANVAS_HEIGHT, CANVAS_WIDTH, ROI_SIZE, TILES
from evaluate import compute_psnr, evaluate_roi, parse_methods, render_readme_section, update_readme
from generate_chart import _mosaic_bayer, build_svg, export_tiffs
from mtf import compute_slanted_edge_mtf, evaluate_mtf
from scipy.special import erf

if TYPE_CHECKING:
    from pathlib import Path


def test_tile_rois_are_even_and_non_overlapping() -> None:
    assert len(TILES) == 16
    seen_coords: set[tuple[int, int]] = set()
    for tile in TILES:
        x0, y0, w, h = tile.roi_bounds()
        assert x0 % 2 == 0
        assert y0 % 2 == 0
        assert w == ROI_SIZE
        assert h == ROI_SIZE
        assert x0 >= 0
        assert x0 + w <= CANVAS_WIDTH
        assert y0 >= 0
        assert y0 + h <= CANVAS_HEIGHT
        assert (tile.row, tile.col) not in seen_coords
        seen_coords.add((tile.row, tile.col))


def test_export_tiffs_creates_five_float32_tiffs_with_unclamped_values(tmp_path: Path) -> None:
    svg_path = tmp_path / "chart.svg"
    build_svg().write(svg_path, encoding="utf-8", xml_declaration=True)
    export_tiffs(svg_path=svg_path, temp_dir=tmp_path)

    gt = tifffile.imread(tmp_path / "ground_truth_rgb.tif")
    assert gt.dtype == np.float32
    assert gt.shape == (CANVAS_HEIGHT, CANVAS_WIDTH, 3)
    assert float(gt.min()) < -0.2
    assert float(gt.max()) > 3.5

    for pattern in BAYER_PATTERNS:
        bayer = tifffile.imread(tmp_path / f"bayer_{pattern}.tif")
        assert bayer.dtype == np.float32
        assert bayer.shape == (CANVAS_HEIGHT, CANVAS_WIDTH)
        assert float(bayer.min()) < -0.2
        assert float(bayer.max()) > 3.5


def test_mosaic_bayer_matches_pattern_offsets() -> None:
    rgb = np.zeros((4, 4, 3), dtype=np.float32)
    rgb[..., 0] = 1.0  # R
    rgb[..., 1] = 2.0  # G
    rgb[..., 2] = 3.0  # B
    rggb = _mosaic_bayer(rgb, "rggb")
    assert (rggb[0, 0], rggb[0, 1], rggb[1, 0], rggb[1, 1]) == (1.0, 2.0, 2.0, 3.0)
    bggr = _mosaic_bayer(rgb, "bggr")
    assert (bggr[0, 0], bggr[0, 1], bggr[1, 0], bggr[1, 1]) == (3.0, 2.0, 2.0, 1.0)


def test_evaluate_roi_exact_analytical_metrics() -> None:
    tile = TILES[13]  # signed_zero_crossing [-0.25, +0.25]
    gt = np.linspace(-0.25, 0.25, 32 * 32 * 3, dtype=np.float32).reshape(32, 32, 3)
    identical = evaluate_roi(gt, gt, tile)
    assert identical["mae"] == 0.0
    assert identical["mse"] == 0.0
    assert math.isinf(float(identical["psnr"]))
    assert math.isclose(float(identical["ssim"]), 1.0, rel_tol=1e-6)

    shifted = gt + np.float32(0.05)
    m = evaluate_roi(shifted, gt, tile)
    assert math.isclose(float(m["mae"]), 0.05, rel_tol=1e-5)
    assert math.isclose(float(m["mse"]), 0.0025, rel_tol=1e-5)
    expected_psnr = compute_psnr(0.0025, 0.5)
    assert math.isclose(float(m["psnr"]), expected_psnr, rel_tol=1e-5)


def test_parse_methods_and_readme_update_idempotent(tmp_path: Path) -> None:
    assert parse_methods("all") == ["linear", "ppg"]
    assert parse_methods("ppg,linear,ppg") == ["ppg", "linear"]

    readme = tmp_path / "README.md"
    readme.write_text(
        "# Title\n\n<!-- compare-demosaic-results:start -->\nold\n<!-- compare-demosaic-results:end -->\n",
        encoding="utf-8",
    )
    summary = [
        {
            "method": "linear",
            "pattern": "rggb",
            "category": "clean_srgb",
            "mae": 0.01,
            "mse": 0.001,
            "rmse": 0.0316,
            "max_ae": 0.2,
            "psnr": 30.0,
            "psnr_r": 29.0,
            "psnr_g": 32.0,
            "psnr_b": 29.0,
            "ssim": 0.95,
            "chroma_rg_mae": 0.01,
            "chroma_bg_mae": 0.01,
            "chroma_rg_std": 0.01,
            "chroma_bg_std": 0.01,
            "gt_min": 0.0,
            "gt_max": 1.0,
            "est_min": 0.0,
            "est_max": 1.0,
        }
    ]
    tiles = [{"tile_id": TILES[0].id, **summary[0]}]
    sec = render_readme_section(summary, tiles)
    update_readme(readme, sec)
    first = readme.read_text(encoding="utf-8")
    update_readme(readme, sec)
    assert readme.read_text(encoding="utf-8") == first
    assert "| `linear` | `clean_srgb` |" in first


def test_compute_slanted_edge_mtf_matches_analytical_gaussian() -> None:
    size = 192
    sigma = 0.8
    slope = math.tan(math.radians(5.0))
    yy, xx = np.mgrid[:size, :size]
    d_perp = (xx - (size / 2.0 + slope * (yy - size / 2.0))) / math.hypot(1.0, slope)

    for lo, hi in ((0.0, 1.0), (-0.25, 0.25)):
        patch = lo + (hi - lo) * 0.5 * (1.0 + erf(d_perp / (math.sqrt(2.0) * sigma)))
        res = compute_slanted_edge_mtf(patch)
        mask = (res.freqs >= 0.05) & (res.freqs <= 0.45)
        expected = np.exp(-2.0 * (math.pi**2) * (sigma**2) * (res.freqs[mask] ** 2))
        mae = float(np.mean(np.abs(res.mtf[mask] - expected)))
        assert mae < 0.03
        expected_mtf50 = math.sqrt(math.log(2.0) / (2.0 * (math.pi**2) * (sigma**2)))
        assert abs(res.mtf50 - expected_mtf50) < 0.02


def test_evaluate_mtf_writes_svg_charts(tmp_path: Path) -> None:
    gt = np.zeros((CANVAS_HEIGHT, CANVAS_WIDTH, 3), dtype=np.float32)
    # Put a 5-deg step in each slanted-edge ROI so centroid fitting succeeds
    slope = math.tan(math.radians(5.0))
    for tile in TILES:
        if tile.id in {"line_pairs_edges", "signed_zero_crossing", "wide_range_signed_hdr"}:
            x0, y0, w, h = tile.roi_bounds()
            yy, xx = np.mgrid[:h, :w]
            edge = ((xx % (w // 2)) >= (w // 4) + slope * (yy % (h // 2))).astype(np.float32)
            gt[y0 : y0 + h, x0 : x0 + w, :] = edge[..., None]
    tifffile.imwrite(tmp_path / "ground_truth_rgb.tif", gt, photometric="rgb")
    lin_dir = tmp_path / "linear"
    lin_dir.mkdir(parents=True, exist_ok=True)
    for pat in BAYER_PATTERNS:
        tifffile.imwrite(lin_dir / f"demosaic_{pat}.tif", gt, photometric="rgb")

    rows = evaluate_mtf(temp_dir=tmp_path, methods=["linear"], gt_rgb=gt, write_charts=True)
    assert len(rows) > 0
    comp_svg = tmp_path / "mtf_comparison.svg"
    pat_svg = lin_dir / "mtf_rggb.svg"
    assert comp_svg.exists()
    assert "<svg" in comp_svg.read_text(encoding="utf-8")
    assert pat_svg.exists()
    assert "<svg" in pat_svg.read_text(encoding="utf-8")
