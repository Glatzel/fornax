#!/usr/bin/env python3
"""Generate the synthetic demosaic comparison SVG chart and float32 TIFFs."""

from __future__ import annotations

import math
import os
import random
import subprocess
import tempfile
import xml.etree.ElementTree as ET
from pathlib import Path
from typing import Annotated

import numpy as np
import tifffile
import typer
from chart_spec import (
    BAYER_PATTERNS,
    CANVAS_HEIGHT,
    CANVAS_WIDTH,
    HEADER_HEIGHT,
    NOISE_SEED,
    NOISY_PATCH_SPECS,
    ROI_SIZE,
    SUPERSAMPLE_FACTOR,
    TILE_SIZE,
    TILES,
    TileSpec,
)
from PIL import Image

CRATE_DIR = Path(__file__).resolve().parent.parent
REPO_ROOT = CRATE_DIR.parent.parent
DEFAULT_TEMP_DIR = REPO_ROOT / "temp" / "compare-demosaic"
DEFAULT_SVG_PATH = DEFAULT_TEMP_DIR / "demosaic_chart.svg"

FONT_FAMILY = "DejaVu Sans, Liberation Sans, Noto Sans, Arial, sans-serif"

app = typer.Typer(no_args_is_help=True)


def _el(parent: ET.Element, tag: str, **attrs: str | float) -> ET.Element:
    str_attrs = {k.replace("_", "-"): str(v) for k, v in attrs.items()}
    return ET.SubElement(parent, tag, str_attrs)


def _add_tile_chrome(root: ET.Element, defs: ET.Element, tile: TileSpec) -> ET.Element:
    tx, ty = tile.tile_origin()
    rx, ry, rw, rh = tile.roi_bounds()

    # Tile background & subtle border
    _el(
        root,
        "rect",
        x=tx,
        y=ty,
        width=TILE_SIZE,
        height=TILE_SIZE,
        fill="#16181d",
        stroke="#2f333d",
        stroke_width=1,
    )
    # Header bar
    _el(
        root,
        "rect",
        x=tx,
        y=ty,
        width=TILE_SIZE,
        height=HEADER_HEIGHT,
        fill="#22252d",
    )
    _el(
        root,
        "line",
        x1=tx,
        y1=ty + HEADER_HEIGHT,
        x2=tx + TILE_SIZE,
        y2=ty + HEADER_HEIGHT,
        stroke="#363b47",
        stroke_width=1,
    )

    # Tile title & badge
    title_el = _el(
        root,
        "text",
        x=tx + 14,
        y=ty + 25,
        fill="#eef1f6",
        font_family=FONT_FAMILY,
        font_size=14,
        font_weight="bold",
    )
    title_el.text = f"{tile.row * 4 + tile.col + 1:02d} · {tile.title}"

    badge_color = {
        "clean_srgb": "#8ab4f8",
        "noisy_mosaic": "#f6ad55",
        "unclamped_float": "#c084fc",
    }[tile.category]
    badge_el = _el(
        root,
        "text",
        x=tx + TILE_SIZE - 14,
        y=ty + 25,
        fill=badge_color,
        font_family=FONT_FAMILY,
        font_size=12,
        text_anchor="end",
    )
    badge_el.text = tile.badge

    # ROI clip path and group
    clip_id = f"clip-{tile.id}"
    clip = _el(defs, "clipPath", id=clip_id)
    _el(clip, "rect", x=rx, y=ry, width=rw, height=rh)

    # Subtle ROI frame behind content
    _el(
        root,
        "rect",
        x=rx - 1,
        y=ry - 1,
        width=rw + 2,
        height=rh + 2,
        fill="#000000",
        stroke="#3f4554",
        stroke_width=1,
    )
    return _el(root, "g", clip_path=f"url(#{clip_id})")


def _draw_siemens_star(g: ET.Element, rx: int, ry: int, size: int) -> None:
    _el(g, "rect", x=rx, y=ry, width=size, height=size, fill="#ffffff")
    cx = rx + size / 2.0
    cy = ry + size / 2.0
    outer_r = size * 0.48
    spokes = 72
    step = 2.0 * math.pi / spokes
    for i in range(0, spokes, 2):
        a0 = i * step
        a1 = (i + 1) * step
        x1 = cx + outer_r * math.cos(a0)
        y1 = cy + outer_r * math.sin(a0)
        x2 = cx + outer_r * math.cos(a1)
        y2 = cy + outer_r * math.sin(a1)
        d = f"M {cx:.2f} {cy:.2f} L {x1:.2f} {y1:.2f} A {outer_r:.2f} {outer_r:.2f} 0 0 1 {x2:.2f} {y2:.2f} Z"
        _el(g, "path", d=d, fill="#000000")
    for r in (44, 112, 180):
        _el(
            g,
            "circle",
            cx=cx,
            cy=cy,
            r=r,
            fill="none",
            stroke="#808080",
            stroke_width=1.2,
            stroke_dasharray="4 4",
        )
    _el(g, "circle", cx=cx, cy=cy, r=6, fill="#808080")


def _draw_zone_plate(g: ET.Element, rx: int, ry: int, size: int) -> None:
    _el(g, "rect", x=rx, y=ry, width=size, height=size, fill="#808080")
    cx = rx + size / 2.0
    cy = ry + size / 2.0
    max_r = size * 0.49
    # Concentric quadratic-frequency rings representing the analytic zone plate
    n_rings = 40
    for k in range(n_rings, 0, -1):
        r = max_r * math.sqrt(k / n_rings)
        fill = "#ffffff" if k % 2 == 0 else "#000000"
        _el(g, "circle", cx=cx, cy=cy, r=f"{r:.2f}", fill=fill)
    _el(g, "circle", cx=cx, cy=cy, r=14, fill="#808080")


def _draw_line_pairs_edges(g: ET.Element, rx: int, ry: int, size: int) -> None:
    _el(g, "rect", x=rx, y=ry, width=size, height=size, fill="#ffffff")
    half = size // 2

    # Top-left: 5 deg slanted B/W edge
    _el(g, "rect", x=rx, y=ry, width=half, height=half, fill="#f2f2f2")
    _el(
        g,
        "polygon",
        points=f"{rx},{ry} {rx + half // 2 - 10},{ry} {rx + half // 2 + 10},{ry + half} {rx},{ry + half}",
        fill="#0d0d0d",
    )

    # Top-right: 5 deg slanted R/B + mid-gray edge
    tx = rx + half
    _el(g, "rect", x=tx, y=ry, width=half, height=half, fill="#2255dd")
    _el(
        g,
        "polygon",
        points=f"{tx},{ry} {tx + half // 2 + 10},{ry} {tx + half // 2 - 10},{ry + half} {tx},{ry + half}",
        fill="#ee3322",
    )

    # Bottom-left: Horizontal & vertical line pairs (1, 2, 3, 4 px)
    by = ry + half
    _el(g, "rect", x=rx, y=by, width=half, height=half, fill="#ffffff")
    pitches = (1, 2, 3, 4)
    sub = half // 2
    # Horizontal bars (top sub-quadrant)
    for idx, pitch in enumerate(pitches):
        band_x = rx + idx * (sub // 2) + 6
        band_w = (sub // 2) - 10
        y_pos = by + 8
        while y_pos + pitch <= by + sub - 6:
            _el(
                g,
                "rect",
                x=band_x,
                y=y_pos,
                width=band_w,
                height=pitch,
                fill="#000000",
                shape_rendering="crispEdges",
            )
            y_pos += 2 * pitch
    # Vertical bars (bottom sub-quadrant)
    for idx, pitch in enumerate(pitches):
        band_y = by + sub + idx * (sub // 4) + 4
        band_h = (sub // 4) - 6
        x_pos = rx + 8
        while x_pos + pitch <= rx + half - 8:
            _el(
                g,
                "rect",
                x=x_pos,
                y=band_y,
                width=pitch,
                height=band_h,
                fill="#000000",
                shape_rendering="crispEdges",
            )
            x_pos += 2 * pitch

    # Bottom-right: 45 deg diagonal line pairs
    _el(g, "rect", x=tx, y=by, width=half, height=half, fill="#ffffff")
    diag_clip_id = f"clip-diag-{rx}-{ry}"
    # Add a local clipPath inside g via a nested defs? No, clipPath can be inline or we clamp lines to quadrant
    _ = diag_clip_id
    for idx, pitch in enumerate((2, 3, 4, 6)):
        x_start = tx + idx * (half // 4) + 2
        x_end = tx + (idx + 1) * (half // 4) - 2
        step = 2 * pitch * math.sqrt(2)
        d = -float(half)
        while d < half * 2:
            # Line y = by + d - (x - x_start) clipped to [by, by + half]
            # Solve for x at y = by and y = by + half
            y_at_start = by + d
            y_at_end = by + d - (x_end - x_start)
            x0, y0 = float(x_start), y_at_start
            x1, y1 = float(x_end), y_at_end
            if y0 > by + half:
                x0 += y0 - (by + half)
                y0 = float(by + half)
            if y1 < by:
                x1 -= by - y1
                y1 = float(by)
            if x0 <= x1 and by <= y0 <= by + half and by <= y1 <= by + half:
                _el(
                    g,
                    "line",
                    x1=f"{x0:.1f}",
                    y1=f"{y0:.1f}",
                    x2=f"{x1:.1f}",
                    y2=f"{y1:.1f}",
                    stroke="#000000",
                    stroke_width=pitch,
                )
            d += step
    _el(g, "line", x1=rx, y1=by, x2=rx + size, y2=by, stroke="#808080", stroke_width=1)
    _el(g, "line", x1=tx, y1=ry, x2=tx, y2=ry + size, stroke="#808080", stroke_width=1)


def _draw_curved_edges(g: ET.Element, rx: int, ry: int, size: int) -> None:
    _el(g, "rect", x=rx, y=ry, width=size, height=size, fill="#101216")
    cx = rx + size / 2.0
    cy = ry + size / 2.0

    # Concentric alternating rings (decreasing stroke thickness)
    for r, sw, col in (
        (206, 6, "#ffffff"),
        (190, 4, "#e63946"),
        (176, 3, "#ffffff"),
        (164, 2, "#2a9d8f"),
        (154, 1.5, "#ffffff"),
        (146, 1, "#457b9d"),
    ):
        _el(g, "circle", cx=cx, cy=cy, r=r, fill="none", stroke=col, stroke_width=sw)

    # High-contrast quarter-circle discs in corners
    for dx, dy, fill in (
        (rx + 64, ry + 64, "#ffffff"),
        (rx + size - 64, ry + 64, "#ff3333"),
        (rx + 64, ry + size - 64, "#33dd44"),
        (rx + size - 64, ry + size - 64, "#3366ff"),
    ):
        _el(g, "circle", cx=dx, cy=dy, r=40, fill=fill)

    # Nested ellipses
    for rx_e, ry_e, sw, col in (
        (120, 68, 3, "#ffffff"),
        (100, 52, 2, "#ffd166"),
        (80, 36, 1.5, "#ffffff"),
    ):
        _el(
            g,
            "ellipse",
            cx=cx,
            cy=cy,
            rx=rx_e,
            ry=ry_e,
            fill="none",
            stroke=col,
            stroke_width=sw,
            transform=f"rotate(-25 {cx} {cy})",
        )

    # Sinusoidal S-curves across the center
    for offset, col, sw in ((-12, "#ffffff", 2.5), (0, "#ff4444", 1.5), (12, "#44aaff", 1.5)):
        pts = []
        for i in range(65):
            t = i / 64.0
            px = rx + 24 + t * (size - 48)
            py = cy + offset + 38 * math.sin(t * 3.0 * math.pi)
            pts.append(f"{px:.1f},{py:.1f}")
        _el(g, "polyline", points=" ".join(pts), fill="none", stroke=col, stroke_width=sw)


def _add_checker_pattern(defs: ET.Element, pat_id: str, pitch: int, color_a: str, color_b: str) -> None:
    p = _el(
        defs,
        "pattern",
        id=pat_id,
        width=2 * pitch,
        height=2 * pitch,
        patternUnits="userSpaceOnUse",
    )
    _el(p, "rect", x=0, y=0, width=2 * pitch, height=2 * pitch, fill=color_a, shape_rendering="crispEdges")
    _el(p, "rect", x=0, y=0, width=pitch, height=pitch, fill=color_b, shape_rendering="crispEdges")
    _el(p, "rect", x=pitch, y=pitch, width=pitch, height=pitch, fill=color_b, shape_rendering="crispEdges")


def _draw_bw_checkerboard(defs: ET.Element, g: ET.Element, rx: int, ry: int, size: int) -> None:
    half = size // 2
    specs = (
        (0, 0, 1, "1 px"),
        (half, 0, 2, "2 px"),
        (0, half, 3, "3 px"),
        (half, half, 4, "4 px"),
    )
    for dx, dy, pitch, label in specs:
        pat_id = f"pat-bw-{pitch}"
        _add_checker_pattern(defs, pat_id, pitch, "#000000", "#ffffff")
        _el(
            g,
            "rect",
            x=rx + dx,
            y=ry + dy,
            width=half,
            height=half,
            fill=f"url(#{pat_id})",
            shape_rendering="crispEdges",
        )
        _el(g, "rect", x=rx + dx + 8, y=ry + dy + 8, width=56, height=22, rx=4, fill="#111318", fill_opacity="0.85")
        t = _el(
            g,
            "text",
            x=rx + dx + 36,
            y=ry + dy + 23,
            fill="#ffffff",
            font_family=FONT_FAMILY,
            font_size=12,
            font_weight="bold",
            text_anchor="middle",
        )
        t.text = label


def _draw_woven_texture(defs: ET.Element, g: ET.Element, rx: int, ry: int, size: int) -> None:
    # 3 vertical bands: Plain weave, 2x2 Twill, Herringbone
    w3 = size // 3
    # 1. Plain weave (4x4 tile)
    p_plain = _el(defs, "pattern", id="pat-weave-plain", width=4, height=4, patternUnits="userSpaceOnUse")
    _el(p_plain, "rect", x=0, y=0, width=4, height=4, fill="#383c44")
    _el(p_plain, "rect", x=0, y=0, width=2, height=1, fill="#d8dce6")
    _el(p_plain, "rect", x=2, y=2, width=2, height=1, fill="#d8dce6")
    _el(p_plain, "rect", x=2, y=0, width=1, height=2, fill="#9aa4b8")
    _el(p_plain, "rect", x=0, y=2, width=1, height=2, fill="#9aa4b8")
    _el(g, "rect", x=rx, y=ry, width=w3, height=size, fill="url(#pat-weave-plain)")

    # 2. Twill 2x2 diagonal (8x8 tile)
    p_twill = _el(defs, "pattern", id="pat-weave-twill", width=8, height=8, patternUnits="userSpaceOnUse")
    _el(p_twill, "rect", x=0, y=0, width=8, height=8, fill="#2e3440")
    for row in range(8):
        col = row % 8
        _el(p_twill, "rect", x=col, y=row, width=4 if col <= 4 else 8 - col, height=1, fill="#c9d1d9")
        if col > 4:
            _el(p_twill, "rect", x=0, y=row, width=col - 4, height=1, fill="#c9d1d9")
    _el(g, "rect", x=rx + w3, y=ry, width=w3, height=size, fill="url(#pat-weave-twill)")

    # 3. Herringbone subtle warm/cool weave (8x16 tile)
    p_herr = _el(defs, "pattern", id="pat-weave-herr", width=8, height=16, patternUnits="userSpaceOnUse")
    _el(p_herr, "rect", x=0, y=0, width=8, height=16, fill="#3a332c")
    for y in range(8):
        _el(p_herr, "rect", x=y % 4, y=y, width=3, height=1, fill="#d9c6b0")
    for y in range(8, 16):
        _el(p_herr, "rect", x=(15 - y) % 4, y=y, width=3, height=1, fill="#b0c4d9")
    _el(g, "rect", x=rx + 2 * w3, y=ry, width=size - 2 * w3, height=size, fill="url(#pat-weave-herr)")

    for i, lbl in enumerate(("Plain", "Twill", "Herringbone")):
        bx = rx + i * w3 + 8
        _el(g, "rect", x=bx, y=ry + 8, width=92, height=22, rx=4, fill="#111318", fill_opacity="0.85")
        t = _el(
            g, "text", x=bx + 46, y=ry + 23, fill="#ffffff", font_family=FONT_FAMILY, font_size=12, text_anchor="middle"
        )
        t.text = lbl


def _draw_freq_chirps(g: ET.Element, rx: int, ry: int, size: int) -> None:
    _el(g, "rect", x=rx, y=ry, width=size, height=size, fill="#808080")
    h3 = size // 3

    # Top third: Horizontal chirp (lines getting denser left -> right)
    x = 0.0
    toggle = True
    while x < size:
        t = x / size
        pitch = max(1.0, 14.0 * (1.0 - 0.92 * t))
        if toggle:
            _el(g, "rect", x=f"{rx + x:.2f}", y=ry, width=f"{pitch:.2f}", height=h3 - 2, fill="#000000")
        else:
            _el(g, "rect", x=f"{rx + x:.2f}", y=ry, width=f"{pitch:.2f}", height=h3 - 2, fill="#ffffff")
        x += pitch
        toggle = not toggle

    # Middle third: Vertical chirp (bars getting denser top -> bottom)
    y = 0.0
    toggle = True
    while y < h3 - 2:
        t = y / (h3 - 2)
        pitch = max(1.0, 12.0 * (1.0 - 0.90 * t))
        _el(
            g,
            "rect",
            x=rx,
            y=f"{ry + h3 + y:.2f}",
            width=size,
            height=f"{pitch:.2f}",
            fill="#ffffff" if toggle else "#000000",
        )
        y += pitch
        toggle = not toggle

    # Bottom third: Diagonal chirp visual
    by = ry + 2 * h3
    _el(g, "rect", x=rx, y=by, width=size, height=size - 2 * h3, fill="#ffffff")
    d = -float(size)
    toggle = True
    while d < size * 1.5:
        t = max(0.0, min(1.0, (d + size * 0.3) / (size * 1.3)))
        pitch = max(1.2, 16.0 * (1.0 - 0.90 * t))
        if toggle:
            _el(
                g,
                "line",
                x1=rx + d,
                y1=by + (size - 2 * h3),
                x2=rx + d + (size - 2 * h3),
                y2=by,
                stroke="#000000",
                stroke_width=f"{pitch:.2f}",
            )
        d += pitch
        toggle = not toggle


def _draw_impulses_grid(g: ET.Element, rx: int, ry: int, size: int) -> None:
    _el(g, "rect", x=rx, y=ry, width=size, height=size, fill="#050507")
    half = size // 2

    # Left half: 1x1 and 2x2 impulses across 4 Bayer phases (0,0), (1,0), (0,1), (1,1)
    colors = ("#ffffff", "#ff0000", "#00ff00", "#0000ff")
    phases = ((0, 0), (1, 0), (0, 1), (1, 1))
    for row_idx, col_hex in enumerate(colors):
        for col_idx, (px, py) in enumerate(phases):
            bx = rx + 32 + col_idx * 48 + px
            by = ry + 48 + row_idx * 96 + py
            # 1x1 impulse
            _el(g, "rect", x=bx, y=by, width=1, height=1, fill=col_hex, shape_rendering="crispEdges")
            # 2x2 impulse below
            _el(g, "rect", x=bx, y=by + 36, width=2, height=2, fill=col_hex, shape_rendering="crispEdges")
            # Subtle finder ring outside impulse
            _el(g, "circle", cx=bx + 0.5, cy=by + 0.5, r=10, fill="none", stroke="#2b303b", stroke_width=0.8)
            _el(g, "circle", cx=bx + 1, cy=by + 37, r=10, fill="none", stroke="#2b303b", stroke_width=0.8)

    # Right half: 1px & 2px colored micro-grid lines
    gx0 = rx + half + 16
    gw = half - 32
    for i, col_hex in enumerate(("#ffffff", "#ff2222", "#22ff22", "#2266ff")):
        y0 = ry + 16 + i * 104
        for step in range(0, gw, 16):
            _el(
                g,
                "line",
                x1=gx0 + step,
                y1=y0,
                x2=gx0 + step,
                y2=y0 + 88,
                stroke=col_hex,
                stroke_width=1 if i % 2 == 0 else 2,
                shape_rendering="crispEdges",
            )
        for step in range(0, 89, 16):
            _el(
                g,
                "line",
                x1=gx0,
                y1=y0 + step,
                x2=gx0 + gw,
                y2=y0 + step,
                stroke=col_hex,
                stroke_width=1 if i % 2 == 0 else 2,
                shape_rendering="crispEdges",
            )


def _draw_chroma_checkerboard(defs: ET.Element, g: ET.Element, rx: int, ry: int, size: int) -> None:
    half = size // 2
    quads = (
        (0, 0, 1, "#ff0000", "#0000ff", "R/B 1px"),
        (half, 0, 2, "#ff0000", "#0000ff", "R/B 2px"),
        (0, half, 1, "#00ff00", "#ff00ff", "G/M 1px"),
        (half, half, 2, "#00ff00", "#ff00ff", "G/M 2px"),
    )
    for idx, (dx, dy, pitch, ca, cb, label) in enumerate(quads):
        pat_id = f"pat-chroma-{idx}"
        _add_checker_pattern(defs, pat_id, pitch, ca, cb)
        _el(
            g,
            "rect",
            x=rx + dx,
            y=ry + dy,
            width=half,
            height=half,
            fill=f"url(#{pat_id})",
            shape_rendering="crispEdges",
        )
        _el(g, "rect", x=rx + dx + 8, y=ry + dy + 8, width=72, height=22, rx=4, fill="#111318", fill_opacity="0.88")
        t = _el(
            g,
            "text",
            x=rx + dx + 44,
            y=ry + dy + 23,
            fill="#ffffff",
            font_family=FONT_FAMILY,
            font_size=12,
            font_weight="bold",
            text_anchor="middle",
        )
        t.text = label


def _draw_color_boundaries(g: ET.Element, rx: int, ry: int, size: int) -> None:
    half = size // 2
    pairs = (
        ("#ff0000", "#0000ff"),
        ("#ff0000", "#00ff00"),
        ("#00ff00", "#0000ff"),
        ("#00ffff", "#ff0000"),
        ("#ff00ff", "#00ff00"),
        ("#ffff00", "#0000ff"),
    )
    band_h = size // len(pairs)
    for i, (c1, c2) in enumerate(pairs):
        y0 = ry + i * band_h
        h = band_h if i < len(pairs) - 1 else size - i * band_h
        _el(g, "rect", x=rx, y=y0, width=half, height=h, fill=c2)
        # 5-deg slanted boundary inside left half
        _el(
            g,
            "polygon",
            points=f"{rx},{y0} {rx + half // 2 - 4},{y0} {rx + half // 2 + 4},{y0 + h} {rx},{y0 + h}",
            fill=c1,
        )

    # Right half: 24-spoke saturated RGBCMY color star
    cx = rx + half + half / 2.0
    cy = ry + size / 2.0
    r_star = half * 0.46
    star_colors = ("#ff0000", "#ffff00", "#00ff00", "#00ffff", "#0000ff", "#ff00ff")
    _el(g, "rect", x=rx + half, y=ry, width=half, height=size, fill="#12141a")
    spokes = 24
    step = 2.0 * math.pi / spokes
    for i in range(spokes):
        a0 = i * step
        a1 = (i + 1) * step
        x1 = cx + r_star * math.cos(a0)
        y1 = cy + r_star * math.sin(a0)
        x2 = cx + r_star * math.cos(a1)
        y2 = cy + r_star * math.sin(a1)
        col = star_colors[i % len(star_colors)]
        d = f"M {cx:.2f} {cy:.2f} L {x1:.2f} {y1:.2f} A {r_star:.2f} {r_star:.2f} 0 0 1 {x2:.2f} {y2:.2f} Z"
        _el(g, "path", d=d, fill=col)


def _draw_gradients_patches(defs: ET.Element, g: ET.Element, rx: int, ry: int, size: int) -> None:
    _el(g, "rect", x=rx, y=ry, width=size, height=size, fill="#1a1c22")
    # Top half: 7 smooth horizontal ramps (Gray, R, G, B, C, M, Y)
    ramps = (
        ("grad-k", "#000000", "#ffffff"),
        ("grad-r", "#000000", "#ff0000"),
        ("grad-g", "#000000", "#00ff00"),
        ("grad-b", "#000000", "#0000ff"),
        ("grad-c", "#000000", "#00ffff"),
        ("grad-m", "#000000", "#ff00ff"),
        ("grad-y", "#000000", "#ffff00"),
    )
    ramp_h = 28
    for i, (gid, c0, c1) in enumerate(ramps):
        lg = _el(defs, "linearGradient", id=gid, x1="0%", y1="0%", x2="100%", y2="0%")
        _el(lg, "stop", offset="0%", stop_color=c0)
        _el(lg, "stop", offset="100%", stop_color=c1)
        _el(g, "rect", x=rx + 8, y=ry + 8 + i * (ramp_h + 2), width=size - 16, height=ramp_h, fill=f"url(#{gid})")

    # Bottom half: 24-patch ColorChecker-inspired grid (6 cols x 4 rows)
    patches = (
        # Row 1: Natural & skin
        "#735244",
        "#c29682",
        "#627a9d",
        "#576c43",
        "#8580b1",
        "#67bdaa",
        # Row 2: Foliage & sky
        "#d67e2c",
        "#505ba6",
        "#c15a63",
        "#5e3c6c",
        "#9dbc40",
        "#e0a32e",
        # Row 3: Primaries & secondaries
        "#383d96",
        "#469449",
        "#af363c",
        "#e7c71f",
        "#bb5695",
        "#0885a1",
        # Row 4: 6-step neutral gray scale
        "#f3f3f2",
        "#c8c8c8",
        "#a0a0a0",
        "#7a7a7a",
        "#555555",
        "#343434",
    )
    py0 = ry + 224
    pw = (size - 16) // 6
    ph = (size - 232) // 4
    for idx, col in enumerate(patches):
        r, c = divmod(idx, 6)
        _el(
            g,
            "rect",
            x=rx + 8 + c * pw + 3,
            y=py0 + r * ph + 3,
            width=pw - 6,
            height=ph - 6,
            fill=col,
            rx=2,
        )


def _draw_dead_leaves(g: ET.Element, rx: int, ry: int, size: int) -> None:
    _el(g, "rect", x=rx, y=ry, width=size, height=size, fill="#808080")
    rng = random.Random(20261124)
    half = size // 2
    # Left half: achromatic dead leaves; Right half: colored dead leaves
    for is_color in (False, True):
        x_min = rx + (half if is_color else 0)
        sub_g = _el(g, "g")
        for _ in range(240):
            # Power-law radius in [4, 72]
            u = rng.random()
            rad = max(4.0, min(72.0, 4.0 / (u**0.55 + 0.05)))
            cx = x_min + rng.uniform(0, half)
            cy = ry + rng.uniform(0, size)
            if is_color:
                r = rng.randint(15, 245)
                gr = rng.randint(15, 245)
                b = rng.randint(15, 245)
                fill = f"#{r:02x}{gr:02x}{b:02x}"
            else:
                v = rng.randint(10, 245)
                fill = f"#{v:02x}{v:02x}{v:02x}"
            if rng.random() < 0.75:
                _el(sub_g, "circle", cx=f"{cx:.1f}", cy=f"{cy:.1f}", r=f"{rad:.1f}", fill=fill)
            else:
                angle = rng.uniform(0, math.pi)
                w = rad * 1.5
                h = rad * 1.1
                _el(
                    sub_g,
                    "rect",
                    x=f"{cx - w / 2:.1f}",
                    y=f"{cy - h / 2:.1f}",
                    width=f"{w:.1f}",
                    height=f"{h:.1f}",
                    fill=fill,
                    transform=f"rotate({math.degrees(angle):.1f} {cx:.1f} {cy:.1f})",
                )
    _el(g, "line", x1=rx + half, y1=ry, x2=rx + half, y2=ry + size, stroke="#22252d", stroke_width=2)


def _draw_noisy_flat_fields(g: ET.Element, rx: int, ry: int, size: int) -> None:
    # 6 patches (3 cols x 2 rows) matching linear luminance values in ground truth
    patches = (
        ("#000000", "L=0.00", "sigma=0.020 (signed)", "#ffffff"),
        ("#414141", "L=0.05", "sigma=0.015", "#ffffff"),
        ("#777777", "L=0.18", "sigma=0.030", "#ffffff"),
        ("#777777", "L=0.18", "sigma=0.060", "#ffffff"),
        ("#bcbcbc", "L=0.50", "sigma=0.040", "#111111"),
        ("#ededed", "L=0.85", "sigma=0.050", "#111111"),
    )
    pw = size // 3
    ph = size // 2
    for idx, (bg, l_lbl, s_lbl, fg) in enumerate(patches):
        r, c = divmod(idx, 3)
        px = rx + c * pw
        py = ry + r * ph
        _el(g, "rect", x=px, y=py, width=pw, height=ph, fill=bg, stroke="#2f333d", stroke_width=1)
        t1 = _el(
            g,
            "text",
            x=px + pw / 2,
            y=py + ph / 2 - 6,
            fill=fg,
            font_family=FONT_FAMILY,
            font_size=14,
            font_weight="bold",
            text_anchor="middle",
        )
        t1.text = l_lbl
        t2 = _el(
            g,
            "text",
            x=px + pw / 2,
            y=py + ph / 2 + 14,
            fill=fg,
            font_family=FONT_FAMILY,
            font_size=11,
            text_anchor="middle",
        )
        t2.text = s_lbl


def _draw_signed_zero_crossing(defs: ET.Element, g: ET.Element, rx: int, ry: int, size: int) -> None:
    # Visual depiction of [-0.25, +0.25]; analytic float32 replaces ROI during export
    lg = _el(defs, "linearGradient", id="grad-signed", x1="0%", y1="0%", x2="100%", y2="0%")
    _el(lg, "stop", offset="0%", stop_color="#3b267a")
    _el(lg, "stop", offset="50%", stop_color="#111216")
    _el(lg, "stop", offset="100%", stop_color="#e8b84a")
    _el(g, "rect", x=rx, y=ry, width=size, height=size // 2, fill="url(#grad-signed)")
    _el(
        g,
        "line",
        x1=rx + size // 2,
        y1=ry,
        x2=rx + size // 2,
        y2=ry + size // 2,
        stroke="#c084fc",
        stroke_width=2,
        stroke_dasharray="6 4",
    )

    # Bottom half: zero-crossing slanted step & sine visualization
    by = ry + size // 2
    _el(g, "rect", x=rx, y=by, width=size, height=size - size // 2, fill="#e8b84a")
    _el(
        g,
        "polygon",
        points=f"{rx},{by} {rx + size // 2 - 18},{by} {rx + size // 2 + 18},{ry + size} {rx},{ry + size}",
        fill="#3b267a",
    )
    _el(g, "rect", x=rx + 12, y=ry + 12, width=size - 24, height=28, rx=5, fill="#0e1015", fill_opacity="0.82")
    t = _el(
        g,
        "text",
        x=rx + size / 2,
        y=ry + 31,
        fill="#e9d5ff",
        font_family=FONT_FAMILY,
        font_size=13,
        font_weight="bold",
        text_anchor="middle",
    )
    t.text = "Analytic f32 [-0.25, +0.25] (0.0 at center)"


def _draw_hdr_specular(defs: ET.Element, g: ET.Element, rx: int, ry: int, size: int) -> None:
    # Visual depiction of [0.0, 4.0] HDR highlights
    lg = _el(defs, "linearGradient", id="grad-hdr", x1="0%", y1="0%", x2="100%", y2="0%")
    _el(lg, "stop", offset="0%", stop_color="#000000")
    _el(lg, "stop", offset="25%", stop_color="#ffffff")
    _el(lg, "stop", offset="100%", stop_color="#fff7cc")
    _el(g, "rect", x=rx, y=ry, width=size, height=size // 3, fill="url(#grad-hdr)")
    # Mark 1.0 SDR ceiling at 25% width
    x_sdr = rx + size // 4
    _el(
        g,
        "line",
        x1=x_sdr,
        y1=ry,
        x2=x_sdr,
        y2=ry + size // 3,
        stroke="#f97316",
        stroke_width=2,
        stroke_dasharray="5 3",
    )

    # Dark 0.05 floor with 4.0 specular dots & slanted bars
    by = ry + size // 3
    _el(g, "rect", x=rx, y=by, width=size, height=size - size // 3, fill="#38383c")
    for idx, col in enumerate(("#ffffff", "#ff6666", "#66ff66", "#6699ff")):
        cx = rx + 64 + idx * 104
        cy = by + 95
        _el(g, "circle", cx=cx, cy=cy, r=18, fill=col, stroke="#fde047", stroke_width=2)
        _el(g, "line", x1=cx - 28, y1=cy + 85, x2=cx + 28, y2=cy + 145, stroke=col, stroke_width=4)

    _el(g, "rect", x=rx + 12, y=ry + 12, width=size - 24, height=28, rx=5, fill="#0e1015", fill_opacity="0.82")
    t = _el(
        g,
        "text",
        x=rx + size / 2,
        y=ry + 31,
        fill="#fde68a",
        font_family=FONT_FAMILY,
        font_size=13,
        font_weight="bold",
        text_anchor="middle",
    )
    t.text = "Analytic f32 [0.0, 4.0] (1.0 SDR mark at 25%)"


def _draw_wide_range_signed_hdr(g: ET.Element, rx: int, ry: int, size: int) -> None:
    _el(g, "rect", x=rx, y=ry, width=size, height=size, fill="#1a1c22")
    half = size // 2
    # Quadrant 1: -0.5 <-> +2.5 sharp edge
    _el(g, "rect", x=rx, y=ry, width=half, height=half, fill="#4c1d95")
    _el(g, "rect", x=rx + half // 2, y=ry, width=half - half // 2, height=half, fill="#fef08a")
    # Quadrant 2: Wide-range R/B 2px block preview
    _el(g, "rect", x=rx + half, y=ry, width=half, height=half, fill="#dc2626")
    _el(g, "rect", x=rx + half + half // 2, y=ry, width=half // 2, height=half // 2, fill="#2563eb")
    _el(g, "rect", x=rx + half, y=ry + half // 2, width=half // 2, height=half // 2, fill="#2563eb")
    # Bottom half: mixed-sign RGB patches
    patches = (
        ("#ef4444", "(2.5, -0.3, 0.8)"),
        ("#22c55e", "(-0.4, 2.2, 0.2)"),
        ("#3b82f6", "(0.6, -0.5, 2.5)"),
        ("#e5e7eb", "(-0.5, -0.5, -0.5)"),
    )
    pw = size // 2
    ph = (size - half) // 2
    for idx, (col, lbl) in enumerate(patches):
        r, c = divmod(idx, 2)
        px = rx + c * pw
        py = ry + half + r * ph
        _el(g, "rect", x=px + 4, y=py + 4, width=pw - 8, height=ph - 8, fill=col, rx=4)
        fg = "#111111" if idx == 3 else "#ffffff"
        t = _el(
            g,
            "text",
            x=px + pw / 2,
            y=py + ph / 2 + 4,
            fill=fg,
            font_family=FONT_FAMILY,
            font_size=12,
            font_weight="bold",
            text_anchor="middle",
        )
        t.text = lbl

    _el(g, "rect", x=rx + 12, y=ry + 12, width=size - 24, height=28, rx=5, fill="#0e1015", fill_opacity="0.85")
    t = _el(
        g,
        "text",
        x=rx + size / 2,
        y=ry + 31,
        fill="#e9d5ff",
        font_family=FONT_FAMILY,
        font_size=13,
        font_weight="bold",
        text_anchor="middle",
    )
    t.text = "Analytic f32 [-0.5, +2.5] Mixed-Sign HDR"


def build_svg() -> ET.ElementTree:
    """Build the 2048x2048 16-tile synthetic demosaic SVG chart."""
    root = ET.Element(
        "svg",
        {
            "xmlns": "http://www.w3.org/2000/svg",
            "viewBox": f"0 0 {CANVAS_WIDTH} {CANVAS_HEIGHT}",
            "width": str(CANVAS_WIDTH),
            "height": str(CANVAS_HEIGHT),
        },
    )
    defs = _el(root, "defs")
    _el(root, "rect", x=0, y=0, width=CANVAS_WIDTH, height=CANVAS_HEIGHT, fill="#111317")

    for tile in TILES:
        g = _add_tile_chrome(root, defs, tile)
        rx, ry, size, _ = tile.roi_bounds()
        assert size == ROI_SIZE

        if tile.id == "siemens_star":
            _draw_siemens_star(g, rx, ry, size)
        elif tile.id == "zone_plate":
            _draw_zone_plate(g, rx, ry, size)
        elif tile.id == "line_pairs_edges":
            _draw_line_pairs_edges(g, rx, ry, size)
        elif tile.id == "curved_edges":
            _draw_curved_edges(g, rx, ry, size)
        elif tile.id == "bw_checkerboard":
            _draw_bw_checkerboard(defs, g, rx, ry, size)
        elif tile.id == "woven_texture":
            _draw_woven_texture(defs, g, rx, ry, size)
        elif tile.id == "freq_chirps":
            _draw_freq_chirps(g, rx, ry, size)
        elif tile.id == "impulses_grid":
            _draw_impulses_grid(g, rx, ry, size)
        elif tile.id == "chroma_checkerboard":
            _draw_chroma_checkerboard(defs, g, rx, ry, size)
        elif tile.id == "color_boundaries":
            _draw_color_boundaries(g, rx, ry, size)
        elif tile.id == "gradients_patches":
            _draw_gradients_patches(defs, g, rx, ry, size)
        elif tile.id == "dead_leaves":
            _draw_dead_leaves(g, rx, ry, size)
        elif tile.id == "noisy_flat_fields":
            _draw_noisy_flat_fields(g, rx, ry, size)
        elif tile.id == "signed_zero_crossing":
            _draw_signed_zero_crossing(defs, g, rx, ry, size)
        elif tile.id == "hdr_specular":
            _draw_hdr_specular(defs, g, rx, ry, size)
        elif tile.id == "wide_range_signed_hdr":
            _draw_wide_range_signed_hdr(g, rx, ry, size)
        else:
            msg = f"Unhandled tile id: {tile.id}"
            raise ValueError(msg)

    ET.indent(root, space="  ")
    return ET.ElementTree(root)


@app.command()
def svg(
    output: Annotated[Path, typer.Option("--output", "-o", help="Output SVG path.")] = DEFAULT_SVG_PATH,
) -> None:
    """Generate the synthetic SVG test chart."""
    output.parent.mkdir(parents=True, exist_ok=True)
    tree = build_svg()
    tree.write(output, encoding="utf-8", xml_declaration=True)
    typer.echo(f"wrote SVG chart to {output} ({output.stat().st_size} bytes)")


def _find_fonts_dir() -> Path | None:
    """Locate a directory containing TTF/OTF fonts for resvg text rendering."""
    candidates: list[Path] = []
    conda_prefix = os.environ.get("CONDA_PREFIX")
    if conda_prefix:
        candidates.append(Path(conda_prefix) / "fonts")
    candidates.extend(
        [
            Path("/home/glatzel/.pixi/envs/base/fonts"),
            Path("/usr/share/fonts"),
            Path("/usr/local/share/fonts"),
        ]
    )
    for candidate in candidates:
        if candidate.is_dir() and (any(candidate.rglob("*.ttf")) or any(candidate.rglob("*.otf"))):
            return candidate
    return None


def _srgb_to_linear(srgb: np.ndarray) -> np.ndarray:
    """Exact IEC 61966-2-1 sRGB to linear float32 EOTF."""
    return np.where(
        srgb <= 0.04045,
        srgb / 12.92,
        np.power((srgb + 0.055) / 1.055, 2.4),
    ).astype(np.float32)


def linear_to_srgb_u8(linear: np.ndarray) -> np.ndarray:
    """Tone-map/clamp linear float RGB in [0, 1] to 8-bit sRGB for preview PNGs."""
    clamped = np.clip(linear, 0.0, 1.0)
    srgb = np.where(
        clamped <= 0.0031308,
        clamped * 12.92,
        1.055 * np.power(clamped, 1.0 / 2.4) - 0.055,
    )
    return np.clip(np.round(srgb * 255.0), 0, 255).astype(np.uint8)


def _ss_coords(size: int, factor: int = SUPERSAMPLE_FACTOR) -> tuple[np.ndarray, np.ndarray]:
    """Return supersampled (yy, xx) pixel-center coordinates in [0, size)."""
    sub = (np.arange(size * factor, dtype=np.float64) + 0.5) / factor
    return np.meshgrid(sub, sub, indexing="ij")


def _downsample_box(arr: np.ndarray, size: int, factor: int = SUPERSAMPLE_FACTOR) -> np.ndarray:
    """4x4 box (area-average) downsample from (size*factor, size*factor, ...) to (size, size, ...)."""
    if arr.ndim == 2:
        return arr.reshape(size, factor, size, factor).mean(axis=(1, 3)).astype(np.float32)
    c = arr.shape[2]
    return arr.reshape(size, factor, size, factor, c).mean(axis=(1, 3)).astype(np.float32)


def _synth_zone_plate(size: int) -> np.ndarray:
    yy, xx = _ss_coords(size)
    cx = size / 2.0
    cy = size / 2.0
    r2 = (xx - cx) ** 2 + (yy - cy) ** 2
    # DC at center -> Nyquist (0.5 cyc/px) at r = size/2
    val = 0.5 * (1.0 + np.cos(math.pi * 0.5 * r2 / (size / 2.0)))
    gray = _downsample_box(val, size)
    return np.repeat(gray[..., None], 3, axis=2)


def _make_checker(
    h: int, w: int, pitch: int, col_a: tuple[float, float, float], col_b: tuple[float, float, float]
) -> np.ndarray:
    yy, xx = np.ogrid[:h, :w]
    mask = ((xx // pitch) + (yy // pitch)) % 2 == 0
    out = np.empty((h, w, 3), dtype=np.float32)
    out[mask] = col_a
    out[~mask] = col_b
    return out


def _synth_bw_checkerboard(size: int) -> np.ndarray:
    half = size // 2
    out = np.empty((size, size, 3), dtype=np.float32)
    out[:half, :half] = _make_checker(half, half, 1, (1.0, 1.0, 1.0), (0.0, 0.0, 0.0))
    out[:half, half:] = _make_checker(half, half, 2, (1.0, 1.0, 1.0), (0.0, 0.0, 0.0))
    out[half:, :half] = _make_checker(half, half, 3, (1.0, 1.0, 1.0), (0.0, 0.0, 0.0))
    out[half:, half:] = _make_checker(half, half, 4, (1.0, 1.0, 1.0), (0.0, 0.0, 0.0))
    return out


def _synth_chroma_checkerboard(size: int) -> np.ndarray:
    half = size // 2
    out = np.empty((size, size, 3), dtype=np.float32)
    out[:half, :half] = _make_checker(half, half, 1, (1.0, 0.0, 0.0), (0.0, 0.0, 1.0))
    out[:half, half:] = _make_checker(half, half, 2, (1.0, 0.0, 0.0), (0.0, 0.0, 1.0))
    out[half:, :half] = _make_checker(half, half, 1, (0.0, 1.0, 0.0), (1.0, 0.0, 1.0))
    out[half:, half:] = _make_checker(half, half, 2, (0.0, 1.0, 0.0), (1.0, 0.0, 1.0))
    return out


def _synth_freq_chirps(size: int) -> np.ndarray:
    yy, xx = _ss_coords(size)
    h3 = size // 3
    f_max = 0.5  # Nyquist (cycles / pixel)
    out_ss = np.empty((size * SUPERSAMPLE_FACTOR, size * SUPERSAMPLE_FACTOR), dtype=np.float64)

    # Horizontal chirp: phase = 2*pi * 0.5 * f_max * x^2 / size
    b0 = h3 * SUPERSAMPLE_FACTOR
    b1 = 2 * h3 * SUPERSAMPLE_FACTOR
    x_h = xx[:b0, :]
    out_ss[:b0, :] = 0.5 * (1.0 + np.cos(math.pi * f_max * (x_h**2) / size))

    # Vertical chirp
    y_v = yy[b0:b1, :] - h3
    out_ss[b0:b1, :] = 0.5 * (1.0 + np.cos(math.pi * f_max * (y_v**2) / h3))

    # Diagonal 45 deg chirp
    u = ((xx[b1:, :] + (yy[b1:, :] - 2 * h3)) / math.sqrt(2.0)).clip(min=0.0)
    u_max = (size + (size - 2 * h3)) / math.sqrt(2.0)
    out_ss[b1:, :] = 0.5 * (1.0 + np.cos(math.pi * f_max * (u**2) / u_max))

    gray = _downsample_box(out_ss, size)
    return np.repeat(gray[..., None], 3, axis=2)


def _synth_impulses_grid(size: int) -> np.ndarray:
    out = np.full((size, size, 3), 0.02, dtype=np.float32)
    half = size // 2
    colors = (
        (1.0, 1.0, 1.0),
        (1.0, 0.0, 0.0),
        (0.0, 1.0, 0.0),
        (0.0, 0.0, 1.0),
    )
    phases = ((0, 0), (1, 0), (0, 1), (1, 1))
    for r_idx, col in enumerate(colors):
        for c_idx, (px, py) in enumerate(phases):
            bx = 32 + c_idx * 48 + px
            by = 48 + r_idx * 96 + py
            out[by, bx] = col
            out[by + 36 : by + 38, bx : bx + 2] = col

    gx0 = half + 16
    gw = half - 32
    for idx, col in enumerate(colors):
        y0 = 16 + idx * 104
        w = 1 if idx % 2 == 0 else 2
        for step in range(0, gw, 16):
            out[y0 : y0 + 88, gx0 + step : gx0 + step + w] = col
        for step in range(0, 89, 16):
            out[y0 + step : y0 + step + w, gx0 : gx0 + gw] = col
    return out


def _synth_noisy_flat_fields_gt(size: int) -> np.ndarray:
    out = np.empty((size, size, 3), dtype=np.float32)
    pw = size // 3
    ph = size // 2
    for idx, (lum, _) in enumerate(NOISY_PATCH_SPECS):
        r, c = divmod(idx, 3)
        y0, y1 = r * ph, (r + 1) * ph if r == 0 else size
        x0, x1 = c * pw, (c + 1) * pw if c < 2 else size
        out[y0:y1, x0:x1] = lum
    return out


def _synth_signed_zero_crossing(size: int) -> np.ndarray:
    half = size // 2
    out = np.empty((size, size, 3), dtype=np.float32)

    # Top half: linear ramp [-0.25, +0.25]
    ramp = np.linspace(-0.25, 0.25, size, dtype=np.float32)
    out[:half, :, :] = ramp[None, :, None]

    # Bottom-left: 5 deg slanted edge (-0.25 <-> +0.25), 4x supersampled
    yy, xx = _ss_coords(half)
    slope = math.tan(math.radians(5.0))
    boundary = half / 2.0 + slope * (yy - half / 2.0)
    edge_ss = np.where(xx < boundary, -0.25, 0.25)
    edge = _downsample_box(edge_ss, half)
    out[half:, :half, :] = edge[..., None]

    # Bottom-right: signed sine grating 0.25 * sin(2*pi*x/12)
    sine_ss = 0.25 * np.sin(2.0 * math.pi * xx / 12.0)
    sine = _downsample_box(sine_ss, half)
    out[half:, half:, :] = sine[..., None]
    return out


def _synth_hdr_specular(size: int) -> np.ndarray:
    h3 = size // 3
    out = np.full((size, size, 3), 0.05, dtype=np.float32)

    # Top third: linear ramp 0.0 -> 4.0
    ramp = np.linspace(0.0, 4.0, size, dtype=np.float32)
    out[:h3, :, :] = ramp[None, :, None]

    # Bottom 2/3: 4x supersampled specular circles + slanted bars at 4.0
    bh = size - h3
    sub_y = (np.arange(bh * SUPERSAMPLE_FACTOR, dtype=np.float64) + 0.5) / SUPERSAMPLE_FACTOR
    sub_x = (np.arange(size * SUPERSAMPLE_FACTOR, dtype=np.float64) + 0.5) / SUPERSAMPLE_FACTOR
    yy, xx = np.meshgrid(sub_y, sub_x, indexing="ij")
    ss = np.full((bh * SUPERSAMPLE_FACTOR, size * SUPERSAMPLE_FACTOR, 3), 0.05, dtype=np.float64)

    specs = (
        (4.0, 4.0, 4.0),
        (4.0, 0.1, 0.1),
        (0.1, 4.0, 0.1),
        (0.1, 0.1, 4.0),
    )
    for idx, col in enumerate(specs):
        cx = 64.0 + idx * 104.0
        cy = 95.0
        disk_mask = (xx - cx) ** 2 + (yy - cy) ** 2 <= 18.0**2
        ss[disk_mask] = col

        # Slanted 45 deg bar of half-width 2.0 px
        x0, y0 = cx - 28.0, cy + 85.0
        x1, y1 = cx + 28.0, cy + 145.0
        dx, dy = x1 - x0, y1 - y0
        len2 = dx * dx + dy * dy
        t = np.clip(((xx - x0) * dx + (yy - y0) * dy) / len2, 0.0, 1.0)
        dist2 = (xx - (x0 + t * dx)) ** 2 + (yy - (y0 + t * dy)) ** 2
        ss[dist2 <= 2.0**2] = col

    out[h3:, :, :] = (
        ss.reshape(bh, SUPERSAMPLE_FACTOR, size, SUPERSAMPLE_FACTOR, 3).mean(axis=(1, 3)).astype(np.float32)
    )
    return out


def _synth_wide_range_signed_hdr(size: int) -> np.ndarray:
    half = size // 2
    out = np.empty((size, size, 3), dtype=np.float32)

    # Top-left: 5 deg slanted edge (-0.5 <-> +2.5)
    yy, xx = _ss_coords(half)
    slope = math.tan(math.radians(5.0))
    boundary = half / 2.0 + slope * (yy - half / 2.0)
    edge_ss = np.where(xx < boundary, -0.5, 2.5)
    out[:half, :half, :] = _downsample_box(edge_ss, half)[..., None]

    # Top-right: 2px wide-range R/B checkerboard
    out[:half, half:, :] = _make_checker(half, half, 2, (2.5, -0.5, -0.5), (-0.5, -0.5, 2.5))

    # Bottom half: 4 mixed-sign flat patches
    pw = size // 2
    ph = (size - half) // 2
    patches = (
        (2.5, -0.3, 0.8),
        (-0.4, 2.2, 0.2),
        (0.6, -0.5, 2.5),
        (-0.5, -0.5, -0.5),
    )
    for idx, col in enumerate(patches):
        r, c = divmod(idx, 2)
        y0 = half + r * ph
        y1 = half + (r + 1) * ph if r == 0 else size
        x0 = c * pw
        x1 = (c + 1) * pw if c == 0 else size
        out[y0:y1, x0:x1] = col
    return out


def _mosaic_bayer(rgb: np.ndarray, pattern: str) -> np.ndarray:
    """Sample a 3-channel RGB image into a 1-channel Bayer mosaic using BAYER_PATTERNS."""
    mask = BAYER_PATTERNS[pattern]
    h, w, _ = rgb.shape
    mosaic = np.empty((h, w), dtype=np.float32)
    for py in range(2):
        for px in range(2):
            ch = mask[py][px]
            mosaic[py::2, px::2] = rgb[py::2, px::2, ch]
    return mosaic


def _inject_mosaic_noise(mosaic: np.ndarray, tile: TileSpec, rng: np.random.Generator) -> np.ndarray:
    """Inject deterministic Gaussian sensor noise ONLY into noisy_flat_fields of the Bayer mosaic."""
    out = mosaic.copy()
    x0, y0, size, _ = tile.roi_bounds()
    pw = size // 3
    ph = size // 2
    for idx, (_, sigma) in enumerate(NOISY_PATCH_SPECS):
        r, c = divmod(idx, 3)
        py0 = y0 + r * ph
        py1 = y0 + ((r + 1) * ph if r == 0 else size)
        px0 = x0 + c * pw
        px1 = x0 + ((c + 1) * pw if c < 2 else size)
        noise = rng.normal(0.0, sigma, size=(py1 - py0, px1 - px0)).astype(np.float32)
        out[py0:py1, px0:px1] += noise
    return out


def export_tiffs(svg_path: Path, temp_dir: Path) -> None:
    """Rasterize SVG at 4x supersampling, synthesize float32 ROIs, and write 5 TIFFs."""
    if not svg_path.exists():
        svg(output=svg_path)

    temp_dir.mkdir(parents=True, exist_ok=True)
    ss_w = CANVAS_WIDTH * SUPERSAMPLE_FACTOR
    ss_h = CANVAS_HEIGHT * SUPERSAMPLE_FACTOR

    with tempfile.TemporaryDirectory() as tmp:
        ss_png = Path(tmp) / "chart_ss.png"
        cmd = ["resvg", "--width", str(ss_w), "--height", str(ss_h)]
        fonts_dir = _find_fonts_dir()
        if fonts_dir is not None:
            cmd.extend(["--use-fonts-dir", str(fonts_dir)])
        cmd.extend([str(svg_path), str(ss_png)])
        subprocess.run(cmd, check=True)

        Image.MAX_IMAGE_PIXELS = None
        with Image.open(ss_png) as img:
            srgb_ss = np.asarray(img.convert("RGB"), dtype=np.float32) / 255.0

    linear_ss = _srgb_to_linear(srgb_ss)
    gt_rgb = (
        linear_ss.reshape(CANVAS_HEIGHT, SUPERSAMPLE_FACTOR, CANVAS_WIDTH, SUPERSAMPLE_FACTOR, 3)
        .mean(axis=(1, 3))
        .astype(np.float32)
    )

    # Replace analytic, pixel-exact, noisy-flat, and unclamped-float ROIs
    synths = {
        "zone_plate": _synth_zone_plate,
        "bw_checkerboard": _synth_bw_checkerboard,
        "freq_chirps": _synth_freq_chirps,
        "impulses_grid": _synth_impulses_grid,
        "chroma_checkerboard": _synth_chroma_checkerboard,
        "noisy_flat_fields": _synth_noisy_flat_fields_gt,
        "signed_zero_crossing": _synth_signed_zero_crossing,
        "hdr_specular": _synth_hdr_specular,
        "wide_range_signed_hdr": _synth_wide_range_signed_hdr,
    }
    noisy_tile: TileSpec | None = None
    for tile in TILES:
        if tile.id in synths:
            ys, xs = tile.roi_slices()
            gt_rgb[ys, xs, :] = synths[tile.id](ROI_SIZE)
        if tile.id == "noisy_flat_fields":
            noisy_tile = tile

    gt_path = temp_dir / "ground_truth_rgb.tif"
    tifffile.imwrite(gt_path, gt_rgb, photometric="rgb", compression=None)
    typer.echo(f"wrote {gt_path} (shape={gt_rgb.shape}, min={gt_rgb.min():.3f}, max={gt_rgb.max():.3f})")

    preview_path = temp_dir / "ground_truth_preview.png"
    Image.fromarray(linear_to_srgb_u8(gt_rgb)).save(preview_path)

    for idx, pattern in enumerate(BAYER_PATTERNS):
        mosaic = _mosaic_bayer(gt_rgb, pattern)
        if noisy_tile is not None:
            rng = np.random.default_rng(NOISE_SEED + idx)
            mosaic = _inject_mosaic_noise(mosaic, noisy_tile, rng)
        bayer_path = temp_dir / f"bayer_{pattern}.tif"
        tifffile.imwrite(bayer_path, mosaic, photometric="minisblack", compression=None)
        typer.echo(f"wrote {bayer_path} (shape={mosaic.shape}, min={mosaic.min():.3f}, max={mosaic.max():.3f})")


@app.command()
def export(
    svg_path: Annotated[Path, typer.Option("--svg", help="Input SVG path.")] = DEFAULT_SVG_PATH,
    temp_dir: Annotated[Path, typer.Option("--temp-dir", help="Output temp directory.")] = DEFAULT_TEMP_DIR,
) -> None:
    """Export ground truth RGB and 4 Bayer pattern float32 TIFFs."""
    export_tiffs(svg_path=svg_path, temp_dir=temp_dir)


@app.command("all")
def run_all(
    svg_path: Annotated[Path, typer.Option("--svg", help="Output SVG path.")] = DEFAULT_SVG_PATH,
    temp_dir: Annotated[Path, typer.Option("--temp-dir", help="Output temp directory.")] = DEFAULT_TEMP_DIR,
) -> None:
    """Generate SVG and export 5 float32 TIFFs."""
    svg(output=svg_path)
    export(svg_path=svg_path, temp_dir=temp_dir)


if __name__ == "__main__":
    app()
