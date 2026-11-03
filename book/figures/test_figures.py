"""Tests for procedural book figures."""

from __future__ import annotations

import os
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from matplotlib import pyplot as plt

from book.figures.__main__ import figure_scripts, main
from book.figures.common import save_svg


class CommonTest(unittest.TestCase):
    """Exercise the shared SVG output contract."""

    def tearDown(self) -> None:
        """Release figures created by a test."""
        plt.close("all")

    def test_save_svg_writes_pure_vector_text_deterministically(self) -> None:
        """Keep labels as SVG text and remove nondeterministic metadata."""
        with tempfile.TemporaryDirectory() as first_dir:
            first_root = Path(first_dir)
            with patch.dict(os.environ, {"BOOK_FIGURE_ROOT": str(first_root)}):
                fig, ax = plt.subplots()
                ax.text(0.5, 0.5, "R")
                first_path = save_svg(fig, "l01", "sample")
                first_svg = first_path.read_text(encoding="utf-8")

            with tempfile.TemporaryDirectory() as second_dir:
                second_root = Path(second_dir)
                with patch.dict(os.environ, {"BOOK_FIGURE_ROOT": str(second_root)}):
                    second_path = save_svg(fig, "l01", "sample")
                    second_svg = second_path.read_text(encoding="utf-8")

        assert first_path.name == "sample.svg"
        assert first_path.parent.name == "l01"
        assert "<text" in first_svg
        assert "<image" not in first_svg
        assert "data:image" not in first_svg
        assert "<dc:date" not in first_svg
        assert first_svg == second_svg


class RunnerTest(unittest.TestCase):
    """Exercise discovery of figure generator scripts."""

    def tearDown(self) -> None:
        """Release figures created by a test."""
        plt.close("all")

    def test_discovers_only_chapter_prefixed_scripts(self) -> None:
        """Select generator scripts in chapter order and ignore support files."""
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            for name in ("l02_second.py", "l01_first.py", "common.py", "test_figures.py"):
                (root / name).touch()

            assert [path.name for path in figure_scripts(root)] == ["l01_first.py", "l02_second.py"]

    def test_main_generates_sample_figures(self) -> None:
        """Generate both sample figures through the public pixi-task path."""
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            with patch.dict(os.environ, {"BOOK_FIGURE_ROOT": str(root)}):
                main()

            assert (root / "l01" / "bayer-patterns.svg").is_file()
            assert (root / "l02" / "level-normalization.svg").is_file()


class SampleGeneratorTest(unittest.TestCase):
    """Exercise the evidence supplied by the sample generators."""

    def test_sample_generators_record_alt_text(self) -> None:
        """Require descriptive alternative text beside each generated figure."""
        from book.figures import l01_bayer_patterns, l02_level_normalization

        assert "Bayer" in l01_bayer_patterns.ALT_TEXT
        assert "normalized" in l02_level_normalization.ALT_TEXT


if __name__ == "__main__":
    unittest.main()
