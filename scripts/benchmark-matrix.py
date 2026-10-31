#!/usr/bin/env python3
"""Generate the GitHub Actions benchmark matrix."""

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
REGISTRY = ROOT / "crates" / "bench-demosaic" / "algorithms.json"

COMMON_PATHS = [
    "./Cargo.toml",
    "./Cargo.lock",
]
DECODER_PATHS = "\n".join(
    COMMON_PATHS
    + [
        "./crates/bench-decoder/**",
        "./crates/fornax-core/**",
        "./crates/fornax-decoder-rawler/**",
        "./crates/fornax-devtool/**",
        "./crates/libraw/**",
        "./crates/libraw-sys/**",
    ]
)
DEMOSAIC_PATHS = "\n".join(
    COMMON_PATHS
    + [
        "./crates/bench-demosaic/**",
        "./crates/fornax-core/**",
        "./crates/fornax-devtool/**",
        "./crates/fornax-postprocessor/**",
    ]
)
PREPROCESSOR_PATHS = "\n".join(
    COMMON_PATHS
    + [
        "./crates/bench-preprocessor/**",
        "./crates/fornax-core/**",
        "./crates/fornax-devtool/**",
        "./crates/fornax-preprocessor/**",
    ]
)


def load_algorithms() -> list[str]:
    algorithms = json.loads(REGISTRY.read_text(encoding="utf-8"))
    if not algorithms or len(algorithms) != len(set(algorithms)):
        raise ValueError("demosaic algorithm registry must be non-empty and unique")
    if algorithms != sorted(algorithms):
        raise ValueError("demosaic algorithm registry must be alphabetically sorted")
    return algorithms


def benchmark_matrix() -> list[dict[str, str]]:
    matrix = [
        {
            "name": "bench-decoder",
            "package": "bench-decoder",
            "paths": DECODER_PATHS,
        },
    ]
    matrix.extend(
        {
            "name": f"bench-demosaic-{algorithm}",
            "package": "bench-demosaic",
            "algorithm": algorithm,
            "paths": DEMOSAIC_PATHS,
        }
        for algorithm in load_algorithms()
    )
    matrix.append(
        {
            "name": "bench-preprocessor",
            "package": "bench-preprocessor",
            "paths": PREPROCESSOR_PATHS,
        }
    )
    return matrix


if __name__ == "__main__":
    print(json.dumps(benchmark_matrix(), separators=(",", ":")))
