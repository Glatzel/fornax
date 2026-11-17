#!/usr/bin/env python3
"""Print one Git LFS include pattern for the test/perf golden union."""

import csv
from pathlib import Path


def escape(path):
    escaped = []
    for character in path:
        if character in "*?[]":
            escaped.append(f"[{character}]")
        elif character == ",":
            escaped.append("?")
        else:
            escaped.append(character)
    return "".join(escaped)


def main():
    root = Path(__file__).resolve().parents[3]
    paths = set()
    for name in ("test-manifest.csv", "perf-manifest.csv"):
        with (root / "crates/decoder-tracker/golden" / name).open(
            encoding="utf-8", newline=""
        ) as stream:
            for row in csv.DictReader(stream):
                paths.add(row["path"].replace("\\", "/"))
    print(",".join(escape(path) for path in sorted(paths)))


if __name__ == "__main__":
    main()
