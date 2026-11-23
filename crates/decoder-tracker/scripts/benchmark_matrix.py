#!/usr/bin/env python3
"""Build the decoder benchmark GitHub Actions matrix from golden inputs."""

import argparse
import csv
import json
from pathlib import Path

HEADER = ["path", "size", "sha256"]
EXTENDED_HEADER = ["path", "size", "sha256", "decoder_path", "reason", "overhead_only"]
GROUP_HEADER = [*EXTENDED_HEADER, "group"]
GROUPS = frozenset({"core", "full", "overhead", "hdr"})
ALL_FORMATS = frozenset({"3FR", "ARW", "CR3", "CRM", "DNG", "IIQ", "NEF", "RAF", "RW2"})


class GoldenManifestError(ValueError):
    """Invalid golden benchmark manifest."""


def load_manifest(stream):
    reader = csv.DictReader(stream)
    if reader.fieldnames not in (HEADER, EXTENDED_HEADER, GROUP_HEADER):
        msg = "golden manifest header must be path,size,sha256 or include decoder_path,reason,overhead_only and optional group"  # noqa: E501
        raise GoldenManifestError(msg)

    rows = []
    seen = set()
    for line_number, record in enumerate(reader, start=2):
        path = record["path"].replace("\\", "/")
        segments = path.split("/")
        if not path or path.startswith("/") or any(segment in ("", ".", "..") for segment in segments):
            msg = f"line {line_number}: invalid repository path {path!r}"
            raise GoldenManifestError(msg)
        if path in seen:
            msg = f"line {line_number}: duplicate path {path}"
            raise GoldenManifestError(msg)
        seen.add(path)

        try:
            size = int(record["size"])
        except ValueError as error:
            msg = f"line {line_number}: invalid size: {error}"
            raise GoldenManifestError(msg) from error
        if size < 0:
            msg = f"line {line_number}: size must not be negative"
            raise GoldenManifestError(msg)

        sha256 = record["sha256"].lower()
        if len(sha256) != 64 or any(character not in "0123456789abcdef" for character in sha256):
            msg = f"line {line_number}: invalid SHA-256 {sha256!r}"
            raise GoldenManifestError(msg)
        row = {"path": path, "size": size, "sha256": sha256}
        if reader.fieldnames == GROUP_HEADER:
            group = record["group"].strip().lower()
            if group not in GROUPS:
                msg = f"line {line_number}: invalid group {group!r}"
                raise GoldenManifestError(msg)
            row["group"] = group
        rows.append(row)

    if not rows:
        msg = "golden manifest contains no inputs"
        raise GoldenManifestError(msg)
    return rows


def parse_formats(value):
    if value is None or value.strip().lower() == "all":
        return ALL_FORMATS
    formats = frozenset(part.strip().upper() for part in value.split(",") if part.strip())
    unknown = formats - ALL_FORMATS
    if unknown:
        msg = f"unknown decoder benchmark formats: {','.join(sorted(unknown))}"
        raise GoldenManifestError(msg)
    return formats


def parse_groups(value):
    if value is None or value.strip().lower() == "all":
        return None
    groups = frozenset(part.strip().lower() for part in value.split(",") if part.strip())
    unknown = groups - GROUPS
    if unknown:
        msg = f"unknown decoder benchmark groups: {','.join(sorted(unknown))}"
        raise GoldenManifestError(msg)
    return groups


def build_matrix(rows, formats=None, groups=None):
    selected = ALL_FORMATS if formats is None else frozenset(formats)
    include = []
    for row in rows:
        if groups is not None and row.get("group") not in groups:
            continue
        extension = row["path"].rsplit(".", 1)
        if len(extension) != 2 or not extension[1]:
            msg = f"golden path has no extension: {row['path']}"
            raise GoldenManifestError(msg)
        format_name = extension[1].upper()
        if format_name not in selected:
            continue
        if format_name not in ALL_FORMATS:
            msg = f"golden format has no native decoder: {format_name}"
            raise GoldenManifestError(msg)
        include.append(
            {
                "path": row["path"],
                "format": format_name,
                "cargo-options": "--no-default-features",
                "label": f"{format_name}: {row['path']}",
                "size": row["size"],
                "sha256": row["sha256"],
            }
        )
    if not include:
        msg = "decoder benchmark selection contains no golden inputs"
        raise GoldenManifestError(msg)
    return {"include": include}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", type=Path)
    parser.add_argument("--formats", help="comma-separated formats or 'all'")
    parser.add_argument("--group", default="core", help="comma-separated groups or 'all'")
    parser.add_argument("--github-output", type=Path)
    args = parser.parse_args()

    if args.manifest is None:
        parser.error("--manifest is required when building the benchmark matrix")
    with args.manifest.open(encoding="utf-8", newline="") as stream:
        rows = load_manifest(stream)
    matrix = build_matrix(rows, parse_formats(args.formats), parse_groups(args.group))
    rendered = json.dumps(matrix, separators=(",", ":"))
    if args.github_output:
        with args.github_output.open("a", encoding="utf-8") as output:
            output.write(f"matrix={rendered}\n")
    else:
        print(rendered)
    print(f"decoder benchmark matrix: {len(matrix['include'])} inputs")


if __name__ == "__main__":
    main()
