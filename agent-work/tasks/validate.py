#!/usr/bin/env python3
"""Validate the active agent task manifests."""

from __future__ import annotations

import re
import sys
import tomllib
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
TASKS_ROOT = REPO_ROOT / "agent-work" / "tasks"
REQUIRED_FIELDS = {
    "schema_version",
    "id",
    "slug",
    "title",
    "phase",
    "dependencies",
    "deliverables",
    "scope",
    "acceptance",
    "validation",
}
FORBIDDEN_FIELDS = {"status", "state", "review", "approved", "completed"}
PHASES = {"implementation", "integration", "quality"}
ID_RE = re.compile(r"^(IMPL|QA)-[A-Z0-9]+(?:-[A-Z0-9]+)*$")
SLUG_RE = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")


def fail(message: str) -> None:
    print(f"error: {message}", file=sys.stderr)
    raise SystemExit(1)


def load_manifests() -> dict[str, dict[str, object]]:
    manifests: dict[str, dict[str, object]] = {}
    for path in sorted(TASKS_ROOT.glob("*/*.toml")):
        try:
            data = tomllib.loads(path.read_text(encoding="utf-8"))
        except tomllib.TOMLDecodeError as error:
            fail(f"{path.relative_to(REPO_ROOT)}: invalid TOML: {error}")

        missing = REQUIRED_FIELDS - data.keys()
        if missing:
            fail(f"{path.relative_to(REPO_ROOT)}: missing fields: {sorted(missing)}")
        forbidden = FORBIDDEN_FIELDS & data.keys()
        if forbidden:
            fail(f"{path.relative_to(REPO_ROOT)}: remove fields: {sorted(forbidden)}")

        task_id = data["id"]
        if not isinstance(task_id, str) or not ID_RE.fullmatch(task_id):
            fail(f"{path.relative_to(REPO_ROOT)}: invalid task id {task_id!r}")
        if path.stem != task_id:
            fail(f"{path.relative_to(REPO_ROOT)}: filename must be {task_id}.toml")
        if data["schema_version"] != 1:
            fail(f"{task_id}: unsupported schema_version {data['schema_version']!r}")
        if data["phase"] not in PHASES:
            fail(f"{task_id}: unsupported phase {data['phase']!r}")
        if not isinstance(data["slug"], str) or not SLUG_RE.fullmatch(data["slug"]):
            fail(f"{task_id}: invalid slug {data['slug']!r}")

        for field in ("dependencies", "deliverables", "acceptance", "validation"):
            value = data[field]
            if not isinstance(value, list) or not all(isinstance(item, str) for item in value):
                fail(f"{task_id}: {field} must be an array of strings")
        if not data["deliverables"] or not data["acceptance"] or not data["validation"]:
            fail(f"{task_id}: deliverables, acceptance, and validation cannot be empty")
        if not isinstance(data["scope"], str) or not data["scope"].strip():
            fail(f"{task_id}: scope cannot be empty")
        if task_id in manifests:
            fail(f"duplicate task id {task_id}")
        manifests[task_id] = data

    for task_id, data in manifests.items():
        unknown = sorted(set(data["dependencies"]) - manifests.keys())
        if unknown:
            fail(f"{task_id}: unknown dependencies {unknown}")
    return manifests


def main() -> None:
    manifests = load_manifests()
    if not manifests:
        fail("no task manifests found")
    print(f"validated {len(manifests)} task manifests")


if __name__ == "__main__":
    main()
