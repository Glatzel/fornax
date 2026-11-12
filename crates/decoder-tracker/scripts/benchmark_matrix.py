#!/usr/bin/env python3
"""Build the decoder benchmark GitHub Actions matrix from golden inputs."""

import argparse
import csv
import json
from pathlib import Path


HEADER = ["path", "size", "sha256"]


def load_manifest(stream):
    reader = csv.DictReader(stream)
    if reader.fieldnames != HEADER:
        raise ValueError("golden manifest header must be exactly path,size,sha256")

    rows = []
    seen = set()
    for line_number, record in enumerate(reader, start=2):
        path = record["path"].replace("\\", "/")
        segments = path.split("/")
        if (
            not path
            or path.startswith("/")
            or any(segment in ("", ".", "..") for segment in segments)
        ):
            raise ValueError(f"line {line_number}: invalid repository path {path!r}")
        if path in seen:
            raise ValueError(f"line {line_number}: duplicate path {path}")
        seen.add(path)

        try:
            size = int(record["size"])
        except ValueError as error:
            raise ValueError(f"line {line_number}: invalid size: {error}") from error
        if size < 0:
            raise ValueError(f"line {line_number}: size must not be negative")

        sha256 = record["sha256"].lower()
        if len(sha256) != 64 or any(
            character not in "0123456789abcdef" for character in sha256
        ):
            raise ValueError(f"line {line_number}: invalid SHA-256 {sha256!r}")
        rows.append({"path": path, "size": size, "sha256": sha256})

    if not rows:
        raise ValueError("golden manifest contains no inputs")
    return rows


def build_matrix(rows):
    include = []
    for row in rows:
        extension = row["path"].rsplit(".", 1)
        if len(extension) != 2 or not extension[1]:
            raise ValueError(f"golden path has no extension: {row['path']}")
        format_name = extension[1].upper()
        include.append(
            {
                "path": row["path"],
                "format": format_name,
                "label": f"{format_name}: {row['path']}",
                "size": row["size"],
                "sha256": row["sha256"],
            }
        )
    return {"include": include}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", required=True, type=Path)
    parser.add_argument("--github-output", type=Path)
    args = parser.parse_args()

    with args.manifest.open(encoding="utf-8", newline="") as stream:
        rows = load_manifest(stream)
    matrix = build_matrix(rows)
    rendered = json.dumps(matrix, separators=(",", ":"))
    if args.github_output:
        with args.github_output.open("a", encoding="utf-8") as output:
            output.write(f"matrix={rendered}\n")
    else:
        print(rendered)
    print(f"decoder benchmark matrix: {len(rows)} inputs")


if __name__ == "__main__":
    main()
