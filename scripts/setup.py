"""Install Fornax development dependencies and pinned reference sources."""

import argparse
import base64
import hashlib
import json
import shlex
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DARKTABLE_RELEASE = "5.6.2"
DARKTABLE_COMMIT = "20891f6c6fa7be995e5bf9dff9d1ee5062051773"
DARKTABLE_SOURCE = "src/iop/demosaicing/ppg.c"
DARKTABLE_SHA256 = "01ec9ed0feec4c004abc842c6b506cf07b6c45a4510a46f1ac816384d92ad53d"


def run(*command: str) -> None:
    print("+", " ".join(command))
    subprocess.run(command, cwd=ROOT, check=True)


def checkout_darktable() -> Path:
    destination = ROOT / "external" / "darktable" / DARKTABLE_SOURCE
    if destination.is_file():
        content = destination.read_bytes()
        if hashlib.sha256(content).hexdigest() == DARKTABLE_SHA256:
            print(f"darktable {DARKTABLE_RELEASE} source is already available")
            return destination

    darktable_root = ROOT / "external" / "darktable"
    if darktable_root.exists():
        shutil.rmtree(darktable_root)
    destination.parent.mkdir(parents=True, exist_ok=True)

    endpoint = f"repos/darktable-org/darktable/contents/{DARKTABLE_SOURCE}?ref=release-{DARKTABLE_RELEASE}"
    print(f"+ gh api {endpoint} --jq .content")
    encoded = subprocess.run(
        ["gh", "api", endpoint, "--jq", ".content"],
        cwd=ROOT,
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    content = base64.b64decode(encoded)
    actual = hashlib.sha256(content).hexdigest()
    if actual != DARKTABLE_SHA256:
        raise RuntimeError(f"darktable source checksum mismatch: expected {DARKTABLE_SHA256}, got {actual}")
    destination.write_bytes(content)
    print(f"Materialized darktable {DARKTABLE_RELEASE} ({DARKTABLE_COMMIT}) source at {destination}")
    return destination


def environment() -> dict[str, str]:
    pixi_root = ROOT / ".pixi" / "envs" / "dev"
    if sys.platform == "win32":
        return {
            "libraw_root": str(pixi_root / "Library"),
            "path_entry": str(pixi_root / "Library" / "bin"),
            "loader_name": "",
            "loader_path": "",
        }

    loader_name = "DYLD_LIBRARY_PATH" if sys.platform == "darwin" else "LD_LIBRARY_PATH"
    library = str(pixi_root / "lib")
    return {
        "libraw_root": str(pixi_root),
        "path_entry": library,
        "loader_name": loader_name,
        "loader_path": library,
    }


def print_environment(output_format: str) -> None:
    values = environment()
    if output_format == "json":
        print(json.dumps(values))
        return

    print(f"export LIBRAW_ROOT={shlex.quote(values['libraw_root'])}")
    print(f"export PATH={shlex.quote(values['path_entry'])}:${{PATH}}")
    if values["loader_name"]:
        name = values["loader_name"]
        print(f"export {name}={shlex.quote(values['loader_path'])}:${{{name}:-}}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--skip-dng", action="store_true")
    parser.add_argument("--force-dng", action="store_true")
    environment_output = parser.add_mutually_exclusive_group()
    environment_output.add_argument("--env-json", action="store_true", help="print the setup environment as JSON")
    environment_output.add_argument("--env-shell", action="store_true", help="print POSIX shell environment exports")
    args = parser.parse_args()

    if args.env_json:
        print_environment("json")
        return 0
    if args.env_shell:
        print_environment("shell")
        return 0

    run("pixi", "install", "-e", "dev")
    run(
        "git",
        "submodule",
        "update",
        "--init",
        "external/libraw",
        "external/raw-images",
    )
    checkout_darktable()

    if not args.skip_dng:
        command = [sys.executable, str(ROOT / "scripts" / "download_dng_sdk.py")]
        if args.force_dng:
            command.append("--force")
        run(*command)

    print("Setup complete.")
    print('For this POSIX shell, run: eval "$(python3 scripts/setup.py --env-shell)"')
    return 0


if __name__ == "__main__":
    sys.exit(main())
