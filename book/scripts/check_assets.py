# Check the book diff for newly committed binary assets.

from __future__ import annotations

import argparse
import subprocess
import sys


def binary_assets(base_ref: str) -> list[str]:  # numpydoc ignore=PR01,RT01
    """Return binary paths added or changed relative to ``base_ref``."""
    diff = subprocess.run(
        [
            "git",
            "diff",
            "--numstat",
            "--no-renames",
            "--diff-filter=d",
            f"{base_ref}...HEAD",
        ],
        check=True,
        capture_output=True,
        text=True,
    )
    return [line.split("\t", maxsplit=2)[2] for line in diff.stdout.splitlines() if line.startswith("-\t-\t")]


def main(argv: list[str] | None = None) -> int:  # numpydoc ignore=PR01,RT01
    """Run the binary asset check and return a process exit code."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "base_ref",
        nargs="?",
        default="origin/main",
        help="Git revision to compare with (default: origin/main)",
    )
    args = parser.parse_args(argv)

    try:
        paths = binary_assets(args.base_ref)
    except subprocess.CalledProcessError as error:
        message = error.stderr.strip() or f"git diff exited with status {error.returncode}"
        print(message, file=sys.stderr)
        return error.returncode

    if not paths:
        return 0

    print("Binary files cannot be committed to the book:", file=sys.stderr)
    for path in paths:
        print(f"  {path}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
