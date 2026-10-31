#!/usr/bin/env python3
"""Render comparison tables from per-fixture JSON results."""

import argparse
import json
from collections import defaultdict
from pathlib import Path

MARK_BEGIN = "<!-- BEGIN:compare-table -->"
MARK_END = "<!-- END:compare-table -->"
MARKS = {
    "exact": "✅",
    "rounding_only": "🟢",
    "within_tolerance": "⚠️",
    "within_robust_tolerance": "⚠️",
    "differs": "❌",
}


def load_results(directory: Path) -> list[dict]:
    comparisons = []
    for path in sorted(directory.glob("*.json")):
        data = json.loads(path.read_text(encoding="utf-8"))
        comparisons.extend(data["comparisons"])
    if not comparisons:
        raise SystemExit(f"no comparison JSON files in {directory}")
    return comparisons


def render(comparisons: list[dict]) -> str:
    by_algorithm = defaultdict(list)
    for comparison in comparisons:
        by_algorithm[comparison["algorithm"]].append(comparison)

    lines = [MARK_BEGIN]
    for algorithm in sorted(by_algorithm, key=str.lower):
        lines.extend(
            [
                f"### {algorithm}",
                "",
                "| Fixture | Reference Library | Bayer | Verdict | MAE | RMSE | P99 | >1024 |",
                "| --- | --- | --- | --- | ---: | ---: | ---: | ---: |",
            ]
        )
        for row in sorted(
            by_algorithm[algorithm],
            key=lambda item: (item["fixture"].lower(), item["reference_library"].lower()),
        ):
            metrics = row["metrics"]
            mark = MARKS[row["verdict"]["kind"]]
            lines.append(
                f"| {row['fixture']} | [{row['reference_library']}]({row['source_url']}) | "
                f"{row['bayer_pattern']} | {mark} | {metrics['mean_absolute_error']:.3f} | "
                f"{metrics['root_mean_square_error']:.3f} | {metrics['p99_error']} | "
                f"{metrics['over_1024_percent']:.3f}% |"
            )
        lines.append("")
    lines.append(MARK_END)
    return "\n".join(lines)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--results-dir",
        type=Path,
        default=Path(__file__).resolve().parent.parent / "results" / "corpus",
    )
    parser.add_argument(
        "--readme",
        type=Path,
        default=Path(__file__).resolve().parent.parent / "README.md",
    )
    args = parser.parse_args()
    readme = args.readme.read_text(encoding="utf-8")
    start = readme.index(MARK_BEGIN)
    end = readme.index(MARK_END, start) + len(MARK_END)
    rendered = render(load_results(args.results_dir))
    args.readme.write_text(readme[:start] + rendered + readme[end:], encoding="utf-8")


if __name__ == "__main__":
    main()
