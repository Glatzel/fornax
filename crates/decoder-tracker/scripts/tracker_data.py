#!/usr/bin/env python3
"""Run decoder-tracker selection and DuckDB-backed CSV operations."""

import csv
import shutil
from pathlib import Path
from typing import Annotated

import duckdb
import typer
from pathspec import GitIgnoreSpec

SCRIPT_DIR = Path(__file__).resolve().parent
SQL_DIR = SCRIPT_DIR.parent / "sql"
MANIFEST_HEADER = ["path", "size", "sha256"]
BASELINE_HEADER = [
    "status",
    "camera",
    "format",
    "path",
    "decode_pass",
    "raw_mosaic_pass",
    "raw_sha256",
    "process_metadata_pass",
    "handoff_pass",
    "error_kind",
    "width",
    "height",
    "cfa",
    "compression_id",
    "compression_name",
    "compression_lossless",
    "storage",
    "bits_per_sample",
    "sample_format",
    "predictor",
    "rows_per_strip",
    "tile_width",
    "tile_height",
]

app = typer.Typer(no_args_is_help=True)


class TrackerDataError(ValueError):
    """Invalid tracker data operation."""


def sql_string(value: Path | str) -> str:
    return str(value).replace("\\", "/").replace("'", "''")


def validate_header(path: Path, expected: list[str], description: str) -> None:
    try:
        with path.open(encoding="utf-8", newline="") as stream:
            header = next(csv.reader(stream))
    except (OSError, StopIteration) as error:
        message = f"reading {description} {path} failed: {error}"
        raise TrackerDataError(message) from error
    if header != expected:
        message = f"{description} CSV header does not match the canonical schema"
        raise TrackerDataError(message)


def render_sql(name: str, replacements: dict[str, str]) -> str:
    sql = (SQL_DIR / name).read_text(encoding="utf-8")
    for marker, value in replacements.items():
        sql = sql.replace(f"@{marker}@", value)
    return sql


def duckdb_row(sql: str) -> tuple:
    connection = duckdb.connect()
    try:
        row = connection.execute(sql).fetchone()
    except duckdb.Error as error:
        message = f"DuckDB failed: {error}"
        raise TrackerDataError(message) from error
    finally:
        connection.close()
    if row is None:
        message = "DuckDB returned no result row"
        raise TrackerDataError(message)
    return row


def ensure_parent(path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)


def camera_for(path: str) -> str:
    parts = path.split("/")
    if len(parts) < 3:
        message = f"corpus path lacks vendor/camera/filename: {path!r}"
        raise TrackerDataError(message)
    vendor, model = parts[0], parts[1]
    prefix = f"{vendor.lower()} "
    if model.lower() == vendor.lower() or model.lower().startswith(prefix):
        return model
    return f"{vendor} {model}"


def filename_for(path: str) -> str:
    filename = path.rsplit("/", 1)[-1]
    stem, separator, _ = filename.rpartition(".")
    if not separator:
        message = f"corpus filename has no extension: {path!r}"
        raise TrackerDataError(message)
    return stem


def parse_shard(value: str) -> tuple[int, int]:
    try:
        index_text, count_text = value.split("/", 1)
        index = int(index_text)
        count = int(count_text)
    except ValueError as error:
        message = "shard must use zero-based INDEX/COUNT syntax"
        raise TrackerDataError(message) from error
    if count <= 0 or index < 0 or index >= count:
        message = f"shard {index}/{count} is outside the valid range"
        raise TrackerDataError(message)
    return index, count


def select_data(
    manifest: Path,
    output: Path,
    cameras: list[str],
    filenames: list[str],
    globs: list[str],
    shard: tuple[int, int] | None,
) -> int:
    validate_header(manifest, MANIFEST_HEADER, "manifest")
    with manifest.open(encoding="utf-8", newline="") as stream:
        rows = list(csv.DictReader(stream))
    camera_filters = [camera.lower() for camera in cameras]
    filename_filters = [filename.lower() for filename in filenames]
    spec = GitIgnoreSpec.from_lines(globs) if globs else None
    selected = []
    for row in rows:
        path = row["path"]
        if camera_filters and not any(camera in camera_for(path).lower() for camera in camera_filters):
            continue
        if filename_filters and filename_for(path).lower() not in filename_filters:
            continue
        if spec is not None and not spec.match_file(path):
            continue
        selected.append(row)
    selected.sort(key=lambda row: (row["sha256"], row["path"]))
    if shard is not None:
        index, count = shard
        selected = [row for position, row in enumerate(selected) if position % count == index]
    ensure_parent(output)
    with output.open("w", encoding="utf-8", newline="") as stream:
        writer = csv.writer(stream, lineterminator="\n")
        writer.writerow(MANIFEST_HEADER)
        writer.writerows([row["path"], row["size"], row["sha256"]] for row in selected)
    if not selected:
        message = "filters matched no corpus files"
        raise TrackerDataError(message)
    return len(selected)


def merge_data(manifest: Path, shards: list[Path], output: Path) -> int:
    validate_header(manifest, MANIFEST_HEADER, "selected manifest")
    for shard in shards:
        validate_header(shard, BASELINE_HEADER, "shard")
    ensure_parent(output)
    shard_sql = ", ".join(f"'{sql_string(path)}'" for path in shards)
    row = duckdb_row(
        render_sql(
            "merge.sql",
            {
                "MANIFEST": sql_string(manifest),
                "SHARDS": shard_sql,
                "OUTPUT": sql_string(output),
            },
        )
    )
    if len(row) != 9:
        message = "DuckDB merge returned an unexpected summary"
        raise TrackerDataError(message)
    (
        duplicates,
        missing,
        unexpected,
        rows_merged,
        invalid_values,
        invalid_order,
        invalid_metadata,
        invalid_error,
        invalid_status,
    ) = map(int, row)
    if duplicates:
        message = f"merged shards contain {duplicates} duplicate rows"
        raise TrackerDataError(message)
    if missing or unexpected:
        message = f"merged shards are incomplete: {missing} missing and {unexpected} unexpected rows"
        raise TrackerDataError(message)
    if invalid_values or invalid_order or invalid_metadata or invalid_error or invalid_status:
        message = (
            "shard validation failed: "
            f"stages={invalid_values} order={invalid_order} "
            f"metadata={invalid_metadata} errors={invalid_error} "
            f"status={invalid_status}"
        )
        raise TrackerDataError(message)
    validate_header(output, BASELINE_HEADER, "merged baseline")
    return rows_merged


def diff_data(selected: Path, baseline: Path, candidate: Path, output: Path) -> tuple[int, int, int]:
    validate_header(selected, MANIFEST_HEADER, "selected manifest")
    validate_header(baseline, BASELINE_HEADER, "baseline")
    validate_header(candidate, BASELINE_HEADER, "candidate")
    ensure_parent(output)
    row = duckdb_row(
        render_sql(
            "diff.sql",
            {
                "BASELINE": sql_string(baseline),
                "CANDIDATE": sql_string(candidate),
                "SELECTED": sql_string(selected),
                "OUTPUT": sql_string(output),
            },
        )
    )
    return tuple(map(int, row))


def update_data(baseline: Path | None, candidate: Path, output: Path) -> int:
    validate_header(candidate, BASELINE_HEADER, "candidate")
    ensure_parent(output)
    if baseline is None:
        shutil.copyfile(candidate, output)
        with output.open(encoding="utf-8", newline="") as stream:
            return max(sum(1 for _ in csv.reader(stream)) - 1, 0)
    validate_header(baseline, BASELINE_HEADER, "baseline")
    row = duckdb_row(
        render_sql(
            "upsert.sql",
            {
                "BASELINE": sql_string(baseline),
                "CANDIDATE": sql_string(candidate),
                "OUTPUT": sql_string(output),
            },
        )
    )
    validate_header(output, BASELINE_HEADER, "updated baseline")
    return int(row[0])


def migrate_data(baseline: Path, output: Path) -> None:
    ensure_parent(output)
    duckdb_row(
        render_sql(
            "migrate-baseline.sql",
            {
                "BASELINE": sql_string(baseline),
                "OUTPUT": sql_string(output),
            },
        )
    )


def run_command(action):
    try:
        action()
    except TrackerDataError as error:
        typer.echo(f"error: {error}", err=True)
        raise typer.Exit(1) from error


@app.command()
def select(
    manifest: Annotated[Path, typer.Option()],
    output: Annotated[Path, typer.Option("--out")],
    camera: Annotated[list[str] | None, typer.Option()] = None,
    filename: Annotated[list[str] | None, typer.Option()] = None,
    glob: Annotated[list[str] | None, typer.Option()] = None,
    shard: Annotated[str | None, typer.Option()] = None,
):
    def action():
        count = select_data(
            manifest,
            output,
            camera or [],
            filename or [],
            glob or [],
            parse_shard(shard) if shard else None,
        )
        typer.echo(f"selected={count}")

    run_command(action)


@app.command()
def merge(
    manifest: Annotated[Path, typer.Option()],
    output: Annotated[Path, typer.Option("--out")],
    shards: Annotated[list[Path], typer.Argument()],
):
    def action():
        count = merge_data(manifest, shards, output)
        typer.echo(f"merge: wrote {count} complete rows to {output}")

    run_command(action)


@app.command()
def diff(
    selected: Annotated[Path, typer.Option()],
    output: Annotated[Path, typer.Option("--out")],
    baseline: Annotated[Path, typer.Argument()],
    candidate: Annotated[Path, typer.Argument()],
    allow_changes: Annotated[bool, typer.Option()] = False,
):
    def action():
        missing, extra, changed = diff_data(selected, baseline, candidate, output)
        typer.echo(f"diff: {missing} missing, {extra} extra, {changed} changed")
        if (missing or extra or changed) and not allow_changes:
            raise typer.Exit(1)

    run_command(action)


@app.command()
def update(
    candidate: Annotated[Path, typer.Option()],
    output: Annotated[Path, typer.Option("--out")],
    baseline: Annotated[Path | None, typer.Option()] = None,
):
    def action():
        count = update_data(baseline, candidate, output)
        typer.echo(f"update: wrote {count} rows to {output}")

    run_command(action)


@app.command()
def migrate(
    baseline: Annotated[Path, typer.Option()],
    selected: Annotated[Path, typer.Option()],
    corpus_manifest: Annotated[Path, typer.Option()],
    output: Annotated[Path, typer.Option("--out")],
):
    def action():
        with selected.open(encoding="utf-8", newline="") as stream:
            selected_count = sum(1 for _ in csv.DictReader(stream))
        with corpus_manifest.open(encoding="utf-8", newline="") as stream:
            corpus_count = sum(1 for _ in csv.DictReader(stream))
        if selected_count != corpus_count:
            message = (
                "legacy baseline schema migration requires the complete corpus: "
                f"selected {selected_count} of {corpus_count} rows"
            )
            raise TrackerDataError(message)
        migrate_data(baseline, output)
        typer.echo(f"migrate: wrote {output}")

    run_command(action)


if __name__ == "__main__":
    app()
