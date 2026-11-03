"""Generate every procedural figure in chapter order."""

from __future__ import annotations

import runpy
from pathlib import Path


def figure_scripts(root: Path | None = None) -> list[Path]:  # numpydoc ignore=PR01,RT01
    """Return chapter-prefixed generator scripts in deterministic order."""
    script_root = Path(__file__).parent if root is None else root
    return sorted(script_root.glob("l[0-9][0-9]_*.py"))


def main() -> None:  # numpydoc ignore=PR01,RT01
    """Run each figure generator as a script."""
    for script in figure_scripts():
        runpy.run_path(str(script), run_name="__main__")


if __name__ == "__main__":
    main()
