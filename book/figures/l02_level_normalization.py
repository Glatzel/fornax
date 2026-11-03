"""Draw black-level subtraction and white-level normalization."""

from __future__ import annotations

import numpy as np
from matplotlib import pyplot as plt

from book.figures.common import BLUE, GRID, TEXT, configure_matplotlib, save_svg

ALT_TEXT = (
    "A line plot of a digital sensor code mapped to a normalized value. The normalized value "
    "stays at zero below the black level of 512, rises linearly to one at the white level of "
    "4095, and stays at one for larger codes."
)

BLACK_LEVEL = 512
WHITE_LEVEL = 4095


def draw() -> None:
    """Render an example of clipped level normalization."""
    configure_matplotlib()
    digital = np.linspace(0, 4608, 500)
    normalized = np.clip((digital - BLACK_LEVEL) / (WHITE_LEVEL - BLACK_LEVEL), 0, 1)

    figure, axis = plt.subplots(figsize=(8, 4))
    axis.plot(digital, normalized, color=BLUE)
    axis.axvline(BLACK_LEVEL, color=GRID, linestyle=":", linewidth=1)
    axis.axvline(WHITE_LEVEL, color=GRID, linestyle=":", linewidth=1)
    axis.axhline(0, color=GRID, linestyle=":", linewidth=1)
    axis.axhline(1, color=GRID, linestyle=":", linewidth=1)
    axis.set(
        xlim=(0, 4608),
        ylim=(-0.1, 1.1),
        xlabel="Digital code d",
        ylabel="Normalized value v",
        title="v = clip((d - b) / (w - b), 0, 1)",
    )
    axis.text(BLACK_LEVEL + 40, 0.07, "black b = 512", color=TEXT)
    axis.text(WHITE_LEVEL - 40, 0.72, "white w = 4095", color=TEXT, ha="right")
    axis.text(0.55, 0.35, "linear range", color=TEXT, ha="center", transform=axis.transAxes)
    figure.tight_layout()
    save_svg(figure, "l02", "level-normalization")
    plt.close(figure)


if __name__ == "__main__":
    draw()
