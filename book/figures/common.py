"""Shared configuration and output helpers for procedural book figures."""

from __future__ import annotations

import os
from pathlib import Path
from typing import TYPE_CHECKING

from matplotlib import rcParams

if TYPE_CHECKING:
    from matplotlib.figure import Figure

FIGURE_ROOT_ENV = "BOOK_FIGURE_ROOT"
REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_FIGURE_ROOT = REPOSITORY_ROOT / "book" / "src" / "figures"

RED = "#c84a5a"
GREEN = "#2f8f83"
BLUE = "#5271b8"
TEXT = "#767676"
EDGE = "#8a8a8a"
GRID = "#9a9a9a"


def configure_matplotlib() -> None:  # numpydoc ignore=PR01,RT01
    """Apply consistent SVG, typography, and color defaults."""
    rcParams.update(
        {
            "axes.edgecolor": TEXT,
            "axes.labelcolor": TEXT,
            "axes.linewidth": 0.8,
            "figure.facecolor": "none",
            "font.family": "sans-serif",
            "font.size": 10,
            "lines.linewidth": 2,
            "savefig.facecolor": "none",
            "savefig.edgecolor": "none",
            "svg.fonttype": "none",
            "svg.hashsalt": "fornax-book",
            "text.color": TEXT,
            "xtick.color": TEXT,
            "ytick.color": TEXT,
        }
    )


def save_svg(figure: Figure, chapter: str, name: str) -> Path:  # numpydoc ignore=PR01,RT01
    """Save a figure as a transparent SVG under the generated output tree."""
    configure_matplotlib()
    root = Path(os.environ.get(FIGURE_ROOT_ENV, DEFAULT_FIGURE_ROOT))
    path = root / chapter / f"{name}.svg"
    path.parent.mkdir(parents=True, exist_ok=True)
    figure.savefig(
        path,
        format="svg",
        bbox_inches="tight",
        transparent=True,
        metadata={"Date": None},
    )
    return path
