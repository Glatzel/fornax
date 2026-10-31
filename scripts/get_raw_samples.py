"""Download selected raw samples from raw.pixls.us.

Requires: pip install httpx duckdb typer   (and aria2c on PATH)

  get_raw_samples.py '*ILCE-7M5*' --list
  get_raw_samples.py 'Sony - *' 'Nikon - *' -o D:/raws --after 2024-01-01
"""

import re
import shutil
import subprocess
import sys
from dataclasses import dataclass
from datetime import datetime
from fnmatch import fnmatchcase
from pathlib import Path
from typing import Annotated
from urllib.parse import unquote

import duckdb  # ty: ignore[unresolved-import]
import httpx  # ty: ignore[unresolved-import]
import typer  # ty: ignore[unresolved-import]

API = "https://raw.pixls.us/json/getrepository.php?set=all"
DEFAULT_OUT = Path(__file__).resolve().parent.parent / "external" / "raw.pixls.us"
BAD_CHARS = re.compile(r'[<>:"/\\|?*]')
NAME_RE = re.compile(r"^(?P<make>.+?) - (?P<model>.+?) - (?P<rest>.+)$")

app = typer.Typer(add_completion=False, help=__doc__.splitlines()[0])


@dataclass(frozen=True)
class Entry:
    rel: str  # Make/Model/<sha256>.<ext>
    url: str
    sha: str
    make: str | None
    model: str | None
    orig: str  # original filename from the listing


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


# Column 6 is the date added (YYYY-MM-DD); column 7 is an HTML cell holding
# the anchor and the sha256 checksum span.
def extract_rows(json_path: Path) -> list[tuple[str, str, str]]:
    rows = duckdb.execute(
        """
        SELECT DISTINCT
            regexp_extract(cell, 'href=[''"]([^''"]+)[''"]', 1) AS url,
            regexp_extract(cell, '([0-9a-f]{64})', 1)           AS sha256,
            added
        FROM (
            SELECT json_extract_string(je.value, '$[7]') AS cell,
                   json_extract_string(je.value, '$[6]') AS added
            FROM (SELECT content::JSON AS j FROM read_text(?)) AS t,
                 json_each(t.j, '$.data') AS je
        )
        WHERE url <> ''
        """,
        [json_path.as_posix()],
    ).fetchall()
    return [(r[0], r[1], r[2] or "") for r in rows]


def build_entries(rows: list[tuple[str, str, str]], patterns: list[str], after: str | None) -> list[Entry]:
    pats = [p.lower() for p in patterns]
    out: dict[str, Entry] = {}  # keyed by rel; dedupes identical files
    for url, sha, added in rows:
        if after and not (added > after):  # ISO dates compare lexicographically
            continue
        file = unquote(url.split("/")[-1])
        if not any(fnmatchcase(file.lower(), p) for p in pats):
            continue
        stem, dot, ext = file.rpartition(".")
        ext = f".{ext.lower()}" if dot else ""
        stem = stem if dot else file
        name = (sha or safe(stem)) + ext
        if m := NAME_RE.match(stem):
            make, model = safe(m["make"]), safe(m["model"])
            rel = f"{make}/{model}/{name}"
        else:
            make = model = None
            rel = f"_unsorted/{name}"
        out[rel] = Entry(rel, url.replace(" ", "%20"), sha, make, model, file)
    return sorted(out.values(), key=lambda e: e.rel)


def show_table(entries: list[Entry]) -> None:
    con = duckdb.connect()
    con.execute("CREATE TABLE t (make VARCHAR, model VARCHAR, file VARCHAR)")
    con.executemany("INSERT INTO t VALUES (?, ?, ?)", [(e.make, e.model, e.orig) for e in entries])
    con.sql("SELECT make, model, file FROM t").show(
        max_rows=len(entries) + 1,
        max_width=shutil.get_terminal_size().columns,
        max_col_width=200,
        null_value="-",
    )


@app.command()
def main(
    names: Annotated[
        list[str] | None,
        typer.Argument(help="Wildcard(s) on the sample filename, case-insensitive (default: *)."),
    ] = None,
    out: Annotated[Path, typer.Option("-o", "--out", help="Output directory.")] = DEFAULT_OUT,
    after: Annotated[
        datetime | None,
        typer.Option(
            formats=["%Y-%m-%d"],
            help="Only samples added after this date (YYYY-MM-DD, exclusive).",
        ),
    ] = None,
    jobs: Annotated[int, typer.Option("-j", "--jobs", help="Parallel downloads.")] = 8,
    list_only: Annotated[bool, typer.Option("-l", "--list", help="List matches, don't download.")] = False,
    refresh: Annotated[bool, typer.Option("--refresh", help="Ignore the cached listing.")] = False,
    json_file: Annotated[
        Path | None, typer.Option("--json", help="Use a local listing file instead of fetching.")
    ] = None,
) -> None:
    json_path = json_file or fetch_repository(out / ".cache", refresh)
    after_str = after.strftime("%Y-%m-%d") if after else None
    entries = build_entries(extract_rows(json_path), names or ["*"], after_str)

    if not entries:
        typer.echo("No files matched.", err=True)
        raise typer.Exit(1)
    if list_only:
        show_table(entries)
        typer.echo(f"{len(entries)} files")
        return

    aria2 = shutil.which("aria2c")
    if not aria2:
        typer.echo("aria2c not found on PATH", err=True)
        raise typer.Exit(2)

    out.mkdir(parents=True, exist_ok=True)
    input_file = out / "aria2-input.txt"
    lines = []
    for e in entries:
        lines.append(f"{e.url}\n  out={e.rel}\n")
        if e.sha:
            lines.append(f"  checksum=sha-256={e.sha}\n")
    input_file.write_text("".join(lines), encoding="utf-8")

    rc = subprocess.run(
        [
            aria2,
            "-i",
            str(input_file),
            "-d",
            str(out),
            "-j",
            str(jobs),
            "-x",
            "1",
            "-s",
            "1",
            "-c",
            "--auto-file-renaming=false",
            "--file-allocation=none",
        ]
    ).returncode
    raise typer.Exit(rc)


if __name__ == "__main__":
    app()
