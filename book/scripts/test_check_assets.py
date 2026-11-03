"""Tests for the book binary-asset guard."""

from __future__ import annotations

import os
import subprocess
import tempfile
import unittest
from pathlib import Path

from book.scripts import check_assets


class AssetCheckTest(unittest.TestCase):  # numpydoc ignore=PR01,RT01
    """Exercise the checker against real Git diffs."""

    def setUp(self) -> None:
        """Create an isolated repository with a base commit."""
        self._temp_dir = tempfile.TemporaryDirectory()
        self.addCleanup(self._temp_dir.cleanup)
        self.root = Path(self._temp_dir.name)
        previous_cwd = Path.cwd()
        os.chdir(self.root)
        self.addCleanup(os.chdir, previous_cwd)
        self._git("init")
        self._git("config", "user.email", "book@example.invalid")
        self._git("config", "user.name", "Book Test")
        self._write_text("notes.md", "base\n")
        self._commit_and_tag("base")

    def _git(self, *args: str) -> None:
        subprocess.run(
            ["git", *args],
            cwd=self.root,
            check=True,
            capture_output=True,
            text=True,
        )

    def _write_bytes(self, name: str, content: bytes) -> None:
        (self.root / name).write_bytes(content)

    def _write_text(self, name: str, content: str) -> None:
        (self.root / name).write_text(content, encoding="utf-8")

    def _commit_and_tag(self, tag: str) -> None:
        self._git("add", "--all")
        self._git("commit", "-m", tag)
        self._git("tag", tag)

    def test_rejects_added_binary_file(self) -> None:
        """Report a binary file newly present after the base commit."""
        self._write_bytes("image.png", b"\x89PNG\r\n\x1a\n\x00")
        self._git("add", "--all")
        self._git("commit", "-m", "add image")

        assert check_assets.binary_assets("base") == ["image.png"]

    def test_rejects_changed_binary_file(self) -> None:
        """Report a binary file whose contents changed after the base commit."""
        self._write_bytes("asset.raw", b"\x00base")
        self._commit_and_tag("with-binary")
        self._write_bytes("asset.raw", b"\x00changed")
        self._git("add", "--all")
        self._git("commit", "-m", "change binary")

        assert check_assets.binary_assets("with-binary") == ["asset.raw"]

    def test_allows_text_file(self) -> None:
        """Allow a text file added after the base commit."""
        self._write_text("chapter.md", "# Chapter\n")
        self._git("add", "--all")
        self._git("commit", "-m", "add chapter")

        assert check_assets.binary_assets("base") == []

    def test_allows_binary_deletion(self) -> None:
        """Allow removing a binary that predates the comparison base."""
        self._write_bytes("asset.raw", b"\x00base")
        self._commit_and_tag("with-binary")
        (self.root / "asset.raw").unlink()
        self._git("add", "--all")
        self._git("commit", "-m", "delete binary")

        assert check_assets.binary_assets("with-binary") == []


if __name__ == "__main__":
    unittest.main()
