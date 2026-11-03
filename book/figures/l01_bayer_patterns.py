"""Draw the four conventional 2x2 RGB Bayer patterns."""

from __future__ import annotations

from matplotlib import pyplot as plt
from matplotlib.patches import Rectangle

from book.figures.common import BLUE, EDGE, GREEN, RED, TEXT, configure_matplotlib, save_svg

ALT_TEXT = (
    "Four labeled Bayer color-filter arrays. RGGB starts with red at top left and blue at "
    "bottom right; GRBG starts with green, red, blue, then green; GBRG starts with green, "
    "blue, red, then green; and BGGR starts with blue at top left and red at bottom right."
)

PATTERNS = {
    "RGGB": ("R", "G", "G", "B"),
    "GRBG": ("G", "R", "B", "G"),
    "GBRG": ("G", "B", "R", "G"),
    "BGGR": ("B", "G", "G", "R"),
}
COLORS = {"R": RED, "G": GREEN, "B": BLUE}


def draw() -> None:  # numpydoc ignore=PR01,RT01
    """Render the four supported Bayer patterns."""
    configure_matplotlib()
    figure, axes = plt.subplots(1, len(PATTERNS), figsize=(9, 2.8))

    for axis, (pattern, cells) in zip(axes, PATTERNS.items(), strict=True):
        for row in range(2):
            for column in range(2):
                label = cells[row * 2 + column]
                axis.add_patch(
                    Rectangle(
                        (column - 0.5, row - 0.5),
                        1,
                        1,
                        facecolor=COLORS[label],
                        edgecolor=EDGE,
                        linewidth=1,
                    )
                )
                axis.text(
                    column,
                    row,
                    label,
                    ha="center",
                    va="center",
                    color="white",
                    fontsize=15,
                    fontweight="bold",
                )

        axis.set(
            title=pattern,
            xlim=(-0.5, 1.5),
            ylim=(1.5, -0.5),
            aspect="equal",
            xticks=(0, 1),
            yticks=(0, 1),
        )
        axis.set_xticklabels(("column 0", "column 1"))
        axis.set_yticklabels(("row 1", "row 0"))
        axis.tick_params(length=0, labelsize=8)

    figure.suptitle("Conventional 2x2 RGB Bayer patterns", color=TEXT)
    figure.text(
        0.5,
        0.02,
        "Each cell records one color; letters identify the filter.",
        ha="center",
        color=TEXT,
    )
    figure.tight_layout()
    save_svg(figure, "l01", "bayer-patterns")
    plt.close(figure)


if __name__ == "__main__":
    draw()
