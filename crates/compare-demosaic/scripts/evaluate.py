#!/usr/bin/env python3
"""Evaluate demosaiced float32 RGB TIFFs against ground truth and update README/data."""

from __future__ import annotations

import csv
import json
import math
from pathlib import Path
from typing import Annotated

import numpy as np
import tifffile
import typer
from chart_spec import BAYER_PATTERNS, TILES, TileSpec
from generate_chart import linear_to_srgb_u8
from mtf import EDGE_SPECS, evaluate_mtf
from PIL import Image
from skimage.metrics import structural_similarity

CRATE_DIR = Path(__file__).resolve().parent.parent
REPO_ROOT = CRATE_DIR.parent.parent
DEFAULT_TEMP_DIR = REPO_ROOT / "temp" / "compare-demosaic"
DEFAULT_DATA_DIR = DEFAULT_TEMP_DIR
DEFAULT_README_PATH = CRATE_DIR / "README.md"

ALL_METHODS = ("linear", "ppg")
ROI_MARGIN = 4  # Trim 4px inside each 448x448 ROI to avoid border/label edge effects

README_START = "<!-- compare-demosaic-results:start -->"
README_END = "<!-- compare-demosaic-results:end -->"

app = typer.Typer(no_args_is_help=True)


def parse_methods(method_arg: str) -> list[str]:
    """Resolve `--method` string ('all', 'linear', 'ppg', or comma-separated) into a list."""
    tokens = [tok.strip().lower() for tok in method_arg.split(",") if tok.strip()]
    if not tokens or "all" in tokens:
        return list(ALL_METHODS)
    resolved: list[str] = []
    for tok in tokens:
        if tok not in ALL_METHODS:
            msg = f"Unsupported method '{tok}'; expected one of {ALL_METHODS} or 'all'"
            raise typer.BadParameter(msg)
        if tok not in resolved:
            resolved.append(tok)
    return resolved


def compute_psnr(mse: float, data_range: float) -> float:
    """Compute PSNR in dB from MSE and signal data_range."""
    if mse <= 0.0:
        return float("inf")
    return 10.0 * math.log10((data_range * data_range) / mse)


def evaluate_roi(est: np.ndarray, gt: np.ndarray, tile: TileSpec) -> dict[str, float | str]:
    """Compute all accuracy, perceptual, chroma, and range metrics for one tile ROI."""
    diff = est.astype(np.float64) - gt.astype(np.float64)
    abs_diff = np.abs(diff)
    sq_diff = diff * diff

    mae = float(np.mean(abs_diff))
    mse = float(np.mean(sq_diff))
    rmse = math.sqrt(mse)
    max_ae = float(np.max(abs_diff))

    data_range = float(tile.value_range[1] - tile.value_range[0])
    psnr = compute_psnr(mse, data_range)

    ch_psnrs: list[float] = []
    for ch in range(3):
        ch_mse = float(np.mean(sq_diff[..., ch]))
        ch_psnrs.append(compute_psnr(ch_mse, data_range))

    ssim = float(
        structural_similarity(
            gt.astype(np.float64),
            est.astype(np.float64),
            data_range=data_range,
            channel_axis=-1,
            gaussian_weights=True,
            sigma=1.5,
            use_sample_covariance=False,
            win_size=11,
        )
    )

    rg_diff = (est[..., 0].astype(np.float64) - est[..., 1].astype(np.float64)) - (
        gt[..., 0].astype(np.float64) - gt[..., 1].astype(np.float64)
    )
    bg_diff = (est[..., 2].astype(np.float64) - est[..., 1].astype(np.float64)) - (
        gt[..., 2].astype(np.float64) - gt[..., 1].astype(np.float64)
    )
    chroma_rg_mae = float(np.mean(np.abs(rg_diff)))
    chroma_bg_mae = float(np.mean(np.abs(bg_diff)))
    chroma_rg_std = float(np.std(est[..., 0].astype(np.float64) - est[..., 1].astype(np.float64)))
    chroma_bg_std = float(np.std(est[..., 2].astype(np.float64) - est[..., 1].astype(np.float64)))

    return {
        "tile_id": tile.id,
        "tile_title": tile.title,
        "category": tile.category,
        "mae": mae,
        "mse": mse,
        "rmse": rmse,
        "max_ae": max_ae,
        "psnr": psnr,
        "psnr_r": ch_psnrs[0],
        "psnr_g": ch_psnrs[1],
        "psnr_b": ch_psnrs[2],
        "ssim": ssim,
        "chroma_rg_mae": chroma_rg_mae,
        "chroma_bg_mae": chroma_bg_mae,
        "chroma_rg_std": chroma_rg_std,
        "chroma_bg_std": chroma_bg_std,
        "gt_min": float(np.min(gt)),
        "gt_max": float(np.max(gt)),
        "est_min": float(np.min(est)),
        "est_max": float(np.max(est)),
    }


def _trimmed_roi_slices(tile: TileSpec) -> tuple[slice, slice]:
    x0, y0, w, h = tile.roi_bounds()
    return (
        slice(y0 + ROI_MARGIN, y0 + h - ROI_MARGIN),
        slice(x0 + ROI_MARGIN, x0 + w - ROI_MARGIN),
    )


def _aggregate_rows(rows: list[dict[str, float | str]]) -> dict[str, float]:
    """Aggregate per-tile metrics into a category summary."""
    mean_keys = (
        "mae",
        "mse",
        "rmse",
        "psnr",
        "psnr_r",
        "psnr_g",
        "psnr_b",
        "ssim",
        "chroma_rg_mae",
        "chroma_bg_mae",
        "chroma_rg_std",
        "chroma_bg_std",
    )
    agg = {k: float(np.mean([float(r[k]) for r in rows])) for k in mean_keys}
    agg["max_ae"] = float(np.max([float(r["max_ae"]) for r in rows]))
    agg["gt_min"] = float(np.min([float(r["gt_min"]) for r in rows]))
    agg["gt_max"] = float(np.max([float(r["gt_max"]) for r in rows]))
    agg["est_min"] = float(np.min([float(r["est_min"]) for r in rows]))
    agg["est_max"] = float(np.max([float(r["est_max"]) for r in rows]))
    return agg


def _write_previews(method_dir: Path, pattern: str, est: np.ndarray, gt: np.ndarray) -> None:
    preview_path = method_dir / f"preview_{pattern}.png"
    Image.fromarray(linear_to_srgb_u8(est)).save(preview_path)

    # Amplified 8x absolute RGB error heatmap for visual inspection
    err = np.mean(np.abs(est.astype(np.float32) - gt.astype(np.float32)), axis=2)
    amp = np.clip(err * 8.0, 0.0, 1.0)
    # Simple inferno-like false color ramp: black -> purple -> orange -> yellow
    r = np.clip(amp * 2.5, 0.0, 1.0)
    g = np.clip((amp - 0.35) * 2.2, 0.0, 1.0)
    b = np.clip(np.sin(amp * math.pi) * 0.75, 0.0, 1.0)
    heat_u8 = np.clip(np.round(np.stack([r, g, b], axis=2) * 255.0), 0, 255).astype(np.uint8)
    heatmap_path = method_dir / f"error_heatmap_{pattern}.png"
    Image.fromarray(heat_u8).save(heatmap_path)


def _fmt_psnr(val: float) -> str:
    return "inf" if math.isinf(val) else f"{val:.2f}"


def render_readme_section(
    summary_rows: list[dict[str, float | str]],
    tile_rows: list[dict[str, float | str]],
    mtf_rows: list[dict[str, float | str]] | None = None,
) -> str:
    """Render Markdown summary tables for README.md."""
    methods = sorted(
        {str(r["method"]) for r in summary_rows}, key=lambda m: ALL_METHODS.index(m) if m in ALL_METHODS else 99
    )
    lines: list[str] = [
        README_START,
        "",
        "### Overall Summary (Averaged Across All 4 Bayer Patterns)",
        "",
        "| Method | Category | MAE | MSE | PSNR (dB) | PSNR R / G / B (dB) | SSIM | Chroma MAE (R-G / B-G) | MaxAE |",
        "| :--- | :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
    ]
    categories = ("clean_srgb", "unclamped_float", "noisy_mosaic")
    for method in methods:
        for cat in categories:
            matching = [r for r in summary_rows if r["method"] == method and r["category"] == cat]
            if not matching:
                continue
            agg = _aggregate_rows(matching)
            lines.append(
                f"| `{method}` | `{cat}` | {agg['mae']:.6f} | {agg['mse']:.6e} | "
                f"{_fmt_psnr(agg['psnr'])} | "
                f"{_fmt_psnr(agg['psnr_r'])} / {_fmt_psnr(agg['psnr_g'])} / {_fmt_psnr(agg['psnr_b'])} | "
                f"{agg['ssim']:.5f} | "
                f"{agg['chroma_rg_mae']:.6f} / {agg['chroma_bg_mae']:.6f} | "
                f"{agg['max_ae']:.4f} |"
            )

    lines.extend(
        [
            "",
            "### Per-Bayer Pattern Breakdown (`clean_srgb` Tiles 01-12)",
            "",
            "| Method | Pattern | MAE | MSE | PSNR (dB) | PSNR R / G / B (dB) | SSIM |",
            "| :--- | :--- | ---: | ---: | ---: | ---: | ---: |",
        ]
    )
    for r in summary_rows:
        if r["category"] != "clean_srgb":
            continue
        lines.append(
            f"| `{r['method']}` | `{str(r['pattern']).upper()}` | {float(r['mae']):.6f} | {float(r['mse']):.6e} | "
            f"{_fmt_psnr(float(r['psnr']))} | "
            f"{_fmt_psnr(float(r['psnr_r']))} / {_fmt_psnr(float(r['psnr_g']))} / {_fmt_psnr(float(r['psnr_b']))} | "
            f"{float(r['ssim']):.5f} |"
        )

    lines.extend(
        [
            "",
            "### Per-Tile Breakdown (Averaged Across All 4 Bayer Patterns)",
            "",
            "| Tile | Method | MAE | PSNR (dB) | SSIM | Min / Max (GT -> Est) |",
            "| :--- | :--- | ---: | ---: | ---: | :--- |",
        ]
    )
    for tile in TILES:
        for method in methods:
            matching = [r for r in tile_rows if r["tile_id"] == tile.id and r["method"] == method]
            if not matching:
                continue
            agg = _aggregate_rows(matching)
            lines.append(
                f"| {tile.row * 4 + tile.col + 1:02d} · {tile.title} | `{method}` | "
                f"{agg['mae']:.6f} | {_fmt_psnr(agg['psnr'])} | {agg['ssim']:.5f} | "
                f"[{agg['gt_min']:.3f}, {agg['gt_max']:.3f}] -> [{agg['est_min']:.3f}, {agg['est_max']:.3f}] |"
            )

    if mtf_rows:
        lines.extend(
            [
                "",
                "### Slanted-Edge MTF Summary (Averaged Across All 4 Bayer Patterns)",
                "",
                "| Method | Edge | Channel | MTF50 (cyc/px) | MTF20 (cyc/px) | MTF@0.25 | MTF@0.50 | Peak MTF |",
                "| :--- | :--- | :--- | ---: | ---: | ---: | ---: | ---: |",
            ]
        )
        for method in methods:
            for edge in EDGE_SPECS:
                for ch in edge.channels:
                    m_rows = [
                        r for r in mtf_rows if r["method"] == method and r["edge_id"] == edge.id and r["channel"] == ch
                    ]
                    if not m_rows:
                        continue
                    mtf50 = float(np.mean([float(r["mtf50"]) for r in m_rows]))
                    mtf20 = float(np.mean([float(r["mtf20"]) for r in m_rows]))
                    mtf025 = float(np.mean([float(r["mtf_025"]) for r in m_rows]))
                    mtf050 = float(np.mean([float(r["mtf_050"]) for r in m_rows]))
                    mtf_peak = float(np.mean([float(r["mtf_peak"]) for r in m_rows]))
                    lines.append(
                        f"| `{method}` | `{edge.id}` | `{ch}` | "
                        f"{mtf50:.3f} | {mtf20:.3f} | {mtf025:.3f} | {mtf050:.3f} | {mtf_peak:.3f} |"
                    )

    lines.extend(["", README_END])
    return "\n".join(lines)


def update_readme(readme_path: Path, section_md: str) -> None:
    """Replace the marker-bounded results section in README.md."""
    content = readme_path.read_text(encoding="utf-8")
    start_idx = content.find(README_START)
    end_idx = content.find(README_END)
    if start_idx == -1 or end_idx == -1 or end_idx < start_idx:
        msg = f"README markers {README_START} / {README_END} not found in {readme_path}"
        raise ValueError(msg)
    new_content = content[:start_idx] + section_md + content[end_idx + len(README_END) :]
    readme_path.write_text(new_content, encoding="utf-8")


@app.command()
def main(
    method: Annotated[str, typer.Option("--method", help="Method(s): linear, ppg, all, or comma-separated.")] = "all",
    temp_dir: Annotated[Path, typer.Option("--temp-dir", help="Temp directory with TIFFs.")] = DEFAULT_TEMP_DIR,
    data_dir: Annotated[Path, typer.Option("--data-dir", help="Output CSV/JSON directory.")] = DEFAULT_DATA_DIR,
    readme: Annotated[Path, typer.Option("--readme", help="README.md path to update.")] = DEFAULT_README_PATH,
    write_previews: Annotated[bool, typer.Option("--write-previews/--no-write-previews")] = True,
    write_mtf: Annotated[bool, typer.Option("--write-mtf/--no-write-mtf")] = True,
    should_update_readme: Annotated[bool, typer.Option("--update-readme/--no-update-readme")] = True,
) -> None:
    """Evaluate demosaiced outputs against ground_truth_rgb.tif."""
    methods = parse_methods(method)
    gt_path = temp_dir / "ground_truth_rgb.tif"
    gt_rgb = tifffile.imread(gt_path).astype(np.float32)

    tile_rows: list[dict[str, float | str]] = []
    summary_rows: list[dict[str, float | str]] = []

    for m in methods:
        method_dir = temp_dir / m
        for pattern in BAYER_PATTERNS:
            est_path = method_dir / f"demosaic_{pattern}.tif"
            est_rgb = tifffile.imread(est_path).astype(np.float32)
            if est_rgb.shape != gt_rgb.shape:
                msg = f"Shape mismatch for {est_path}: {est_rgb.shape} != {gt_rgb.shape}"
                raise ValueError(msg)

            if write_previews:
                _write_previews(method_dir, pattern, est_rgb, gt_rgb)

            per_pattern_tiles: list[dict[str, float | str]] = []
            for tile in TILES:
                ys, xs = _trimmed_roi_slices(tile)
                metrics = evaluate_roi(est_rgb[ys, xs, :], gt_rgb[ys, xs, :], tile)
                row: dict[str, float | str] = {"method": m, "pattern": pattern, **metrics}
                tile_rows.append(row)
                per_pattern_tiles.append(row)

            for cat in ("clean_srgb", "unclamped_float", "noisy_mosaic"):
                cat_rows = [r for r in per_pattern_tiles if r["category"] == cat]
                agg = _aggregate_rows(cat_rows)
                summary_rows.append({"method": m, "pattern": pattern, "category": cat, **agg})

    mtf_rows = evaluate_mtf(temp_dir=temp_dir, methods=methods, gt_rgb=gt_rgb, write_charts=write_mtf)

    data_dir.mkdir(parents=True, exist_ok=True)
    summary_csv = data_dir / "results.csv"
    with summary_csv.open("w", encoding="utf-8", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=list(summary_rows[0].keys()), lineterminator="\n")
        writer.writeheader()
        writer.writerows(summary_rows)

    tile_csv = data_dir / "tile_results.csv"
    with tile_csv.open("w", encoding="utf-8", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=list(tile_rows[0].keys()), lineterminator="\n")
        writer.writeheader()
        writer.writerows(tile_rows)

    mtf_csv = data_dir / "mtf_results.csv"
    with mtf_csv.open("w", encoding="utf-8", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=list(mtf_rows[0].keys()), lineterminator="\n")
        writer.writeheader()
        writer.writerows(mtf_rows)

    json_path = data_dir / "results.json"
    json_payload = {
        "methods": methods,
        "patterns": list(BAYER_PATTERNS.keys()),
        "resolution": [int(gt_rgb.shape[1]), int(gt_rgb.shape[0])],
        "summary": summary_rows,
        "tiles": tile_rows,
        "mtf": mtf_rows,
    }
    json_path.write_text(json.dumps(json_payload, indent=2) + "\n", encoding="utf-8")

    if should_update_readme:
        section_md = render_readme_section(summary_rows, tile_rows, mtf_rows)
        update_readme(readme, section_md)

    typer.echo(f"wrote {summary_csv}, {tile_csv}, {mtf_csv}, {json_path}")


if __name__ == "__main__":
    app()
