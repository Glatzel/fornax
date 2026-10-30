"""Download and extract the Adobe DNG SDK from Adobe's official download site."""

import argparse
import hashlib
import shutil
import sys
import tempfile
import urllib.request
import zipfile
from pathlib import Path

VERSION = "1.7.1"
BUILD = "2768"
ARCHIVE_NAME = "dng_sdk_1_7_1_2768_20261013.zip"
URL = f"https://download.adobe.com/pub/adobe/dng/{ARCHIVE_NAME}"
SHA256 = "c1d3a545ed45b150e59af2e6aa0695c1551f0e2bed34ad87436898a05194d2b1"
ARCHIVE_ROOT = "dng_sdk_1_7_1"
SDK_DIRECTORY = "dng_sdk"
ROOT = Path(__file__).resolve().parent.parent
DESTINATION = ROOT / "external" / "dng_sdk"
MARKER = DESTINATION / ".version"
REQUIRED_FILE = DESTINATION / "source" / "dng_validate.cpp"


def is_installed() -> bool:
    return (
        REQUIRED_FILE.is_file()
        and MARKER.is_file()
        and MARKER.read_text(encoding="utf-8").strip() == f"{VERSION}/{BUILD}"
    )


def download(archive: Path) -> None:
    digest = hashlib.sha256()
    with urllib.request.urlopen(URL, timeout=180) as response, archive.open("wb") as output:
        while chunk := response.read(1024 * 1024):
            digest.update(chunk)
            output.write(chunk)

    actual = digest.hexdigest()
    if actual != SHA256:
        raise RuntimeError(f"DNG SDK checksum mismatch: expected {SHA256}, downloaded {actual}")


def extract(archive: Path, destination: Path) -> None:
    destination.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix=".dng-sdk-", dir=destination.parent) as temporary:
        temporary_path = Path(temporary)
        with zipfile.ZipFile(archive) as package:
            root = temporary_path.resolve()
            for member in package.infolist():
                target = (temporary_path / member.filename).resolve()
                if not target.is_relative_to(root):
                    raise RuntimeError(f"unsafe DNG SDK archive path: {member.filename}")
            package.extractall(temporary_path)

        source = temporary_path / ARCHIVE_ROOT / SDK_DIRECTORY
        if not (source / "source" / "dng_validate.cpp").is_file():
            raise RuntimeError(f"unexpected DNG SDK archive layout: {source} is incomplete")

        if destination.exists():
            shutil.rmtree(destination)
        shutil.move(source, destination)
        (destination / ".version").write_text(f"{VERSION}/{BUILD}\n", encoding="utf-8")


def install(force: bool = False) -> None:
    if not force and is_installed():
        print(f"DNG SDK {VERSION} build {BUILD} is already installed")
        return

    temporary_archive = DESTINATION.parent / f".{ARCHIVE_NAME}"
    try:
        print(f"Downloading Adobe DNG SDK {VERSION} build {BUILD}")
        download(temporary_archive)
        extract(temporary_archive, DESTINATION)
    finally:
        temporary_archive.unlink(missing_ok=True)
    print(f"Installed Adobe DNG SDK {VERSION} build {BUILD} in {DESTINATION}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    action = parser.add_mutually_exclusive_group()
    action.add_argument("--check", action="store_true", help="verify an existing installation")
    action.add_argument("--force", action="store_true", help="replace an existing installation")
    args = parser.parse_args()

    if args.check:
        if is_installed():
            print(f"DNG SDK {VERSION} build {BUILD} is installed in {DESTINATION}")
            return 0
        print(f"DNG SDK {VERSION} build {BUILD} is not installed", file=sys.stderr)
        return 1

    install(force=args.force)
    return 0


if __name__ == "__main__":
    sys.exit(main())
