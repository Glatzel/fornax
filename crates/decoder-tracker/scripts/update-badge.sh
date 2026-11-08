#!/usr/bin/env bash

set -euo pipefail

crate_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
baseline="$crate_root/baseline.csv"
readme="$crate_root/README.md"
start_marker='<!-- decoder-tracker-badge:start -->'
end_marker='<!-- decoder-tracker-badge:end -->'

while (($# > 0)); do
    case "$1" in
        --baseline)
            baseline="$2"
            shift 2
            ;;
        --readme)
            readme="$2"
            shift 2
            ;;
        *)
            printf 'usage: %s [--baseline FILE] [--readme FILE]\n' "$0" >&2
            exit 2
            ;;
    esac
done

counts="$(
    duckdb -no-agent -csv -noheader -separator ',' :memory: -c "
        SELECT
            count() FILTER (WHERE status = '✅') AS passed,
            count() AS total
        FROM read_csv('$(printf '%s' "$baseline" | sed "s/'/''/g")', header = true);
    "
)"
IFS=, read -r passed total <<<"$counts"
if [[ ! "$passed" =~ ^[0-9]+$ || ! "$total" =~ ^[0-9]+$ ]] || ((total == 0)); then
    printf 'could not derive pass counts from %s\n' "$baseline" >&2
    exit 1
fi

if ((passed == total)); then
    color=success
elif ((passed * 10 >= total * 9)); then
    color=yellow
else
    color=orange
fi

badge="[![DNG corpus: ${passed}/${total} passed](https://img.shields.io/badge/DNG_corpus-${passed}%2F${total}-${color})](baseline.csv)"
starts="$(grep -Fxc "$start_marker" "$readme" || true)"
ends="$(grep -Fxc "$end_marker" "$readme" || true)"
if [[ "$starts" != 1 || "$ends" != 1 ]]; then
    printf '%s must contain exactly one decoder-tracker badge marker pair\n' "$readme" >&2
    exit 1
fi

tmp="$(mktemp "${readme}.XXXXXX")"
trap 'rm -f "$tmp"' EXIT
awk -v start="$start_marker" -v end="$end_marker" -v badge="$badge" '
    $0 == start {
        print start
        print ""
        print badge
        print ""
        skip = 1
        next
    }
    $0 == end {
        skip = 0
        print end
        next
    }
    !skip { print }
' "$readme" >"$tmp"
mv "$tmp" "$readme"
trap - EXIT

printf 'badge: %s/%s passed\n' "$passed" "$total"
