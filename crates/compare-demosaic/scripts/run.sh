#!/usr/bin/env bash
set -euo pipefail

methods="all"
while [[ $# -gt 0 ]]; do
    case "$1" in
        --method|-m)
            methods="$2"
            shift 2
            ;;
        --method=*)
            methods="${1#*=}"
            shift
            ;;
        *)
            printf 'usage: %s [--method linear|ppg|rcd|lmmse|amaze|dht|vng|vng4|all|...]\n' "$0" >&2
            exit 2
            ;;
    esac
done

python crates/compare-demosaic/scripts/generate_chart.py all
cargo run --release -q -p compare-demosaic -- --method "$methods"
python crates/compare-demosaic/scripts/evaluate.py --method "$methods"
