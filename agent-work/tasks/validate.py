#!/usr/bin/env python3
"""Validate agent task manifests and the planned dependency graph."""

from __future__ import annotations

import re
import sys
import tomllib
from collections import defaultdict
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
PHASES = {"research", "synthesis", "design", "implementation", "integration", "quality"}
ID_RE = re.compile(r"^(RES|SYN|DES|IMPL|QA)-[A-Z0-9]+(?:-[A-Z0-9]+)*$")
SLUG_RE = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")


def fail(message: str) -> None:
    print(f"error: {message}", file=sys.stderr)
    raise SystemExit(1)


def load_manifests() -> dict[str, tuple[Path, dict[str, object]]]:
    manifests: dict[str, tuple[Path, dict[str, object]]] = {}
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
            fail(
                f"{path.relative_to(REPO_ROOT)}: GitHub owns live state; "
                f"remove fields: {sorted(forbidden)}"
            )

        task_id = data["id"]
        if not isinstance(task_id, str) or not ID_RE.fullmatch(task_id):
            fail(f"{path.relative_to(REPO_ROOT)}: invalid task id {task_id!r}")
        if path.stem != task_id:
            fail(f"{path.relative_to(REPO_ROOT)}: filename must be {task_id}.toml")
        if data["schema_version"] != 1:
            fail(f"{task_id}: unsupported schema_version {data['schema_version']!r}")
        if data["phase"] not in PHASES:
            fail(f"{task_id}: unknown phase {data['phase']!r}")
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
        manifests[task_id] = (path, data)
    return manifests


def validate_dependencies(manifests: dict[str, tuple[Path, dict[str, object]]]) -> None:
    edges: dict[str, list[str]] = {}
    for task_id, (_, data) in manifests.items():
        dependencies = list(data["dependencies"])
        one_of = data.get("requires_one_of", [])
        if not isinstance(one_of, list) or not all(isinstance(item, str) for item in one_of):
            fail(f"{task_id}: requires_one_of must be an array of strings")
        edges[task_id] = dependencies + one_of
        if task_id in edges[task_id]:
            fail(f"{task_id}: task cannot depend on itself")
        unknown = sorted(set(edges[task_id]) - manifests.keys())
        if unknown:
            fail(f"{task_id}: unknown dependencies {unknown}")

    visiting: set[str] = set()
    visited: set[str] = set()

    def visit(task_id: str, path: list[str]) -> None:
        if task_id in visited:
            return
        if task_id in visiting:
            fail("dependency cycle: " + " -> ".join([*path, task_id]))
        visiting.add(task_id)
        for dependency in edges[task_id]:
            visit(dependency, [*path, task_id])
        visiting.remove(task_id)
        visited.add(task_id)

    for task_id in edges:
        visit(task_id, [])


def validate_ownership(manifests: dict[str, tuple[Path, dict[str, object]]]) -> None:
    owners: dict[str, list[str]] = defaultdict(list)
    for task_id, (_, data) in manifests.items():
        for deliverable in data["deliverables"]:
            owners[deliverable].append(task_id)
            path = REPO_ROOT / deliverable
            if not path.exists() or path.suffix != ".md":
                continue
            front_matter = "\n".join(path.read_text(encoding="utf-8").splitlines()[:30])
            if re.search(r"^> \*\*Status:\*\*", front_matter, flags=re.MULTILINE):
                fail(
                    f"{deliverable}: remove local review status; "
                    f"{task_id} is reviewed in its pull request"
                )

    shared = {path: ids for path, ids in owners.items() if len(ids) > 1}
    if shared:
        fail(f"deliverables have multiple owners: {shared}")


def validate_index(manifests: dict[str, tuple[Path, dict[str, object]]]) -> None:
    index = TASKS_ROOT / "raw-pipeline" / "README.md"
    targets = set(
        re.findall(r"\[`[^`]+`\]\(([^)]+\.toml)\)", index.read_text(encoding="utf-8"))
    )
    indexed_ids = {Path(target).stem for target in targets}
    missing = set(manifests) - indexed_ids
    stale = indexed_ids - set(manifests)
    if missing or stale:
        fail(f"task index mismatch: missing={sorted(missing)} stale={sorted(stale)}")


def main() -> None:
    manifests = load_manifests()
    if not manifests:
        fail("no task manifests found")
    validate_dependencies(manifests)
    validate_ownership(manifests)
    validate_index(manifests)
    print(f"validated {len(manifests)} task manifests")


if __name__ == "__main__":
    main()
