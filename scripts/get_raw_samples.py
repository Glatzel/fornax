"""Download selected raw samples from raw.pixls.us.

Requires: pip install httpx duckdb   (and aria2c on PATH)

  get_raw_samples.py '*ILCE-7M5*' --list
  get_raw_samples.py 'Sony - *' 'Nikon - *' -o D:/raws
"""

import argparse
import re
import shutil
import subprocess
import sys
from fnmatch import fnmatchcase
from pathlib import Path
from urllib.parse import unquote

import duckdb  # ty: ignore[unresolved-import]
import httpx  # ty: ignore[unresolved-import]

API = "https://raw.pixls.us/json/getrepository.php?set=all"
OUT_DIR = Path(__file__).resolve().parent.parent / "external" / "raw.pixls.us"
BAD_CHARS = re.compile(r'[<>:"/\\|?*]')
NAME_RE = re.compile(r"^(?P<make>.+?) - (?P<model>.+?) - (?P<rest>.+)$")


def safe(s: str) -> str:
    return BAD_CHARS.sub("_", s).strip(" .")


# Download the repository listing once; revalidate with ETag afterwards.
def fetch_repository(cache_dir: Path, refresh: bool) -> Path:
    cache_dir.mkdir(parents=True, exist_ok=True)
    data, etag = cache_dir / "repository.json", cache_dir / "repository.etag"

    headers = {}
    if data.exists() and etag.exists() and not refresh:
        headers["If-None-Match"] = etag.read_text().strip()

    try:
        with httpx.Client(timeout=30, follow_redirects=True, http2=False) as client:
            r = client.get(API, headers=headers)
        if r.status_code == 304:
            return data
        r.raise_for_status()
        data.write_bytes(r.content)
        if tag := r.headers.get("etag"):
            etag.write_text(tag)
        else:
            etag.unlink(missing_ok=True)
    except httpx.HTTPError as e:
        if not data.exists():
            raise
        print(f"warning: {e}; using cached listing", file=sys.stderr)
    return data


# Column 7 of each row is an HTML anchor; pull its href out with duckdb.
def extract_urls(json_path: Path) -> list[str]:
    rows = duckdb.execute(
        """
        SELECT DISTINCT
            regexp_extract(json_extract_string(je.value, '$[7]'),
                           'href=[''"]([^''"]+)[''"]', 1) AS url
        FROM (SELECT content::JSON AS j FROM read_text(?)) AS t,
             json_each(t.j, '$.data') AS je
        WHERE url <> ''
        """,
        [json_path.as_posix()],
    ).fetchall()
    return [r[0] for r in rows]


def build_entries(urls: list[str], patterns: list[str]) -> list[tuple[str, str]]:
    pats = [p.lower() for p in patterns]
    out = []
    for url in urls:
        parts = url.split("/")
        file = unquote(parts[-1])
        if not any(fnmatchcase(file.lower(), p) for p in pats):
            continue
        sample_id = parts[-3]  # .../getfile.php/<id>/nice/<name>
        stem, dot, ext = file.rpartition(".")
        ext = dot + ext if dot else ""
        stem = stem if dot else file
        if m := NAME_RE.match(stem):
            leaf = safe(m["rest"] + f" [{sample_id}]" + ext)
            rel = f"{safe(m['make'])}/{safe(m['model'])}/{leaf}"
        else:
            rel = f"_unsorted/{safe(f'{stem} [{sample_id}]{ext}')}"
        out.append((rel, url.replace(" ", "%20")))
    return sorted(out)


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument(
        "name", nargs="*", default=["*"], help="wildcard(s) on the sample filename, case-insensitive (default: *)"
    )
    ap.add_argument("-j", "--jobs", type=int, default=8)
    ap.add_argument("-l", "--list", action="store_true", help="list matches, don't download")
    ap.add_argument("--refresh", action="store_true", help="ignore the cached listing")
    ap.add_argument("--json", type=Path, help="use a local listing file instead of fetching")
    args = ap.parse_args()

    json_path = args.json or fetch_repository(OUT_DIR / ".cache", args.refresh)
    entries = build_entries(extract_urls(json_path), args.name)

    if not entries:
        print("No files matched.", file=sys.stderr)
        return 1
    if args.list:
        for rel, _ in entries:
            print(rel)
        print(f"{len(entries)} files")
        return 0

    aria2 = shutil.which("aria2c")
    if not aria2:
        print("aria2c not found on PATH", file=sys.stderr)
        return 2

    OUT_DIR.mkdir(parents=True, exist_ok=True)
    input_file = OUT_DIR / "aria2-input.txt"
    input_file.write_text("".join(f"{url}\n  out={rel}\n" for rel, url in entries), encoding="utf-8")

    return subprocess.run(
        [
            aria2,
            "-i",
            str(input_file),
            "-d",
            str(OUT_DIR),
            "-j",
            str(args.jobs),
            "-x",
            "1",
            "-s",
            "1",
            "-c",
            "--auto-file-renaming=false",
            "--file-allocation=none",
        ]
    ).returncode


if __name__ == "__main__":
    sys.exit(main())
