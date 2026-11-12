#!/usr/bin/env python3
"""Build the decoder benchmark GitHub Actions matrix from golden inputs."""

import argparse
import csv
import json
from pathlib import Path

HEADER = ["path", "size", "sha256"]
FEATURES = {
    "3FR": "hasselblad",
    "ARW": "sony",
    "CR3": "canon",
    "CRM": "canon",
    "DNG": "",
    "IIQ": "phase-one",
    "NEF": "nikon",
    "RAF": "fujifilm",
    "RW2": "panasonic",
}
ALL_FORMATS = frozenset(FEATURES)
SHARED_PREFIXES = (
    ".github/workflows/ci.yml",
    "Cargo.lock",
    "Cargo.toml",
    "crates/fornax-codec/",
    "crates/fornax-decoder/",
    "crates/fornax-dng/",
    "crates/fornax-raw-core/",
    "crates/fornax-tiff/",
    "crates/decoder-tracker/golden/",
    "crates/decoder-tracker/scripts/benchmark_",
)
VENDOR_FORMATS = (
    ("crates/fornax-raw-canon/", frozenset({"CR3", "CRM"})),
    ("crates/fornax-raw-fujifilm/", frozenset({"RAF"})),
    ("crates/fornax-raw-hasselblad/", frozenset({"3FR"})),
    ("crates/fornax-raw-nikon/", frozenset({"NEF"})),
    ("crates/fornax-raw-panasonic/", frozenset({"RW2"})),
    ("crates/fornax-raw-phase-one/", frozenset({"IIQ"})),
    ("crates/fornax-raw-sony/", frozenset({"ARW"})),
    ("crates/fornax-bmff/", frozenset({"CR3", "CRM"})),
)


class GoldenManifestError(ValueError):
    """Invalid golden benchmark manifest."""


def load_manifest(stream):
    reader = csv.DictReader(stream)
    if reader.fieldnames != HEADER:
        msg = "golden manifest header must be exactly path,size,sha256"
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
        rows.append({"path": path, "size": size, "sha256": sha256})

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


def impacted_formats(paths):
    changed = [path.removeprefix("./").replace("\\", "/") for path in paths if path]
    if not changed:
        return frozenset()
    if any(path == prefix or path.startswith(prefix) for path in changed for prefix in SHARED_PREFIXES):
        return ALL_FORMATS
    selected = set()
    for path in changed:
        for prefix, formats in VENDOR_FORMATS:
            if path.startswith(prefix):
                selected.update(formats)
    return frozenset(selected)


def build_matrix(rows, formats=None):
    selected = ALL_FORMATS if formats is None else frozenset(formats)
    include = []
    for row in rows:
        extension = row["path"].rsplit(".", 1)
        if len(extension) != 2 or not extension[1]:
            msg = f"golden path has no extension: {row['path']}"
            raise GoldenManifestError(msg)
        format_name = extension[1].upper()
        if format_name not in selected:
            continue
        try:
            feature = FEATURES[format_name]
        except KeyError as error:
            msg = f"golden format has no decoder feature: {format_name}"
            raise GoldenManifestError(msg) from error
        include.append(
            {
                "path": row["path"],
                "format": format_name,
                "feature": feature,
                "cargo-options": (
                    f"--no-default-features --features {feature}" if feature else "--no-default-features"
                ),
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
    parser.add_argument("--github-output", type=Path)
    parser.add_argument("--impact", nargs="*", default=None)
    args = parser.parse_args()

    if args.impact is not None:
        formats = impacted_formats(args.impact)
        rendered = ",".join(sorted(formats))
        if args.github_output:
            with args.github_output.open("a", encoding="utf-8") as output:
                output.write(f"formats={rendered}\n")
        else:
            print(rendered)
        return

    if args.manifest is None:
        parser.error("--manifest is required when building the benchmark matrix")
    with args.manifest.open(encoding="utf-8", newline="") as stream:
        rows = load_manifest(stream)
    matrix = build_matrix(rows, parse_formats(args.formats))
    rendered = json.dumps(matrix, separators=(",", ":"))
    if args.github_output:
        with args.github_output.open("a", encoding="utf-8") as output:
            output.write(f"matrix={rendered}\n")
    else:
        print(rendered)
    print(f"decoder benchmark matrix: {len(matrix['include'])} inputs")


if __name__ == "__main__":
    main()
