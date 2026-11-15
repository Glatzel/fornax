#!/usr/bin/env python3
"""Check repository-local links in Markdown and path references in sources."""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
import urllib.parse
from pathlib import Path


MARKDOWN_LINK = re.compile(
    r"(?<!!)\[[^\]]*\]\((?P<target>[^)\s]+)(?:\s+[^)]*)?\)"
)
REPOSITORY_LINK = re.compile(
    r"https://github\.com/Glatzel/fornax/(?:blob|tree)/[^/\s]+/"
    r"(?P<target>docs/[A-Za-z0-9_./@+%?-]+)"
)
DOC_PATH = re.compile(
    r"(?<![A-Za-z0-9_.-])"
    r"(?P<target>(?:\.\.?/)*docs/[A-Za-z0-9_./@+%?-]+)"
)
TEXT_SUFFIXES = {
    ".md",
    ".py",
    ".ps1",
    ".rs",
    ".sh",
    ".toml",
    ".yaml",
    ".yml",
}
TRAILING_PUNCTUATION = ".,;:!?)]}`'\""
HEADING = re.compile(r"^#{1,6}\s+(?P<title>.+?)\s*#*\s*$")
EXPLICIT_ANCHOR = re.compile(r"<a\s+(?:id|name)=[\"'](?P<anchor>[^\"']+)[\"']")


def repository_files(root: Path) -> list[Path]:
    result = subprocess.run(
        ["git", "ls-files", "--cached", "--others", "--exclude-standard", "-z"],
        cwd=root,
        check=True,
        stdout=subprocess.PIPE,
    )
    return [
        root / path
        for path in result.stdout.decode().rstrip("\0").split("\0")
        if path
    ]


def local_target(source: Path, value: str) -> Path | None:
    value = value.strip().strip("<>").split("#", 1)[0].split("?", 1)[0]
    if not value or value.startswith("#"):
        return None
    if re.match(r"^[A-Za-z][A-Za-z0-9+.-]*:", value):
        return None
    return (source.parent / urllib.parse.unquote(value)).resolve()


def markdown_anchors(path: Path) -> set[str]:
    anchors: set[str] = set()
    for line in path.read_text(encoding="utf-8").splitlines():
        anchors.update(match.group("anchor") for match in EXPLICIT_ANCHOR.finditer(line))
        heading = HEADING.match(line)
        if heading is None:
            continue
        title = re.sub(r"<[^>]+>", "", heading.group("title"))
        title = re.sub(r"[`*_~]", "", title).lower()
        title = re.sub(r"[^\w\s-]", "", title)
        anchors.add(re.sub(r"\s+", "-", title.strip()))
    return anchors


def check_target(
    root: Path, source: Path, line: int, value: str, *, repository_rooted: bool = False
) -> str | None:
    value = value.strip().strip("<>")
    target_text, _, fragment = value.partition("#")
    target_text = target_text.split("?", 1)[0]
    if repository_rooted and target_text.startswith("docs/"):
        target = (root / urllib.parse.unquote(target_text)).resolve()
    else:
        target = local_target(source, target_text)
    if target is None:
        return None
    try:
        target.relative_to(root)
    except ValueError:
        return None
    if not target.exists():
        return f"{source.relative_to(root)}:{line}: missing {value}"
    if fragment and target.suffix == ".md":
        fragment = urllib.parse.unquote(fragment)
        if fragment not in markdown_anchors(target):
            return f"{source.relative_to(root)}:{line}: missing fragment {value}"
    return None


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--root",
        type=Path,
        default=Path(__file__).resolve().parent.parent,
    )
    root = parser.parse_args().root.resolve()
    failures: list[str] = []

    for source in repository_files(root):
        if source.suffix not in TEXT_SUFFIXES or not source.is_file():
            continue
        try:
            lines = source.read_text(encoding="utf-8").splitlines()
        except UnicodeDecodeError:
            continue
        for line_number, line in enumerate(lines, 1):
            if source.suffix == ".md":
                for match in MARKDOWN_LINK.finditer(line):
                    failure = check_target(
                        root, source, line_number, match.group("target")
                    )
                    if failure:
                        failures.append(failure)
            patterns = (REPOSITORY_LINK,) if source.suffix == ".md" else (
                REPOSITORY_LINK,
                DOC_PATH,
            )
            for pattern in patterns:
                for match in pattern.finditer(line):
                    if re.search(r"https?://\S*$", line[: match.start()]):
                        continue
                    if match.end() < len(line) and line[match.end()] == "<":
                        continue
                    value = match.group("target").rstrip(TRAILING_PUNCTUATION)
                    if any(marker in value for marker in ("<", ">", "*")):
                        continue
                    failure = check_target(
                        root,
                        source,
                        line_number,
                        value,
                        repository_rooted=True,
                    )
                    if failure:
                        failures.append(failure)

    if failures:
        print("\n".join(sorted(set(failures))), file=sys.stderr)
        return 1
    print("documentation links: ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
