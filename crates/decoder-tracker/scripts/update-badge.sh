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

format_counts="$(
    duckdb -no-agent -csv -noheader -separator ',' :memory: -c "
        SELECT
            format,
            count() FILTER (WHERE status = '✅') AS passed,
            count() AS total
        FROM read_csv('$(printf '%s' "$baseline" | sed "s/'/''/g")', header = true)
        GROUP BY format
        ORDER BY format;
    "
)"

if [[ -z "$format_counts" ]]; then
    printf 'could not derive format counts from %s\n' "$baseline" >&2
    exit 1
fi

passed=0
total=0
format_count=0
first_format=''
table=$'| Format | Status |\n| --- | ---: |'
while IFS=, read -r format format_passed format_total; do
    if [[ -z "$format" || ! "$format_passed" =~ ^[0-9]+$ || ! "$format_total" =~ ^[0-9]+$ ]] ||
        ((format_total == 0 || format_passed > format_total)); then
        printf 'invalid format counts from %s: %s\n' "$baseline" \
            "$format,$format_passed,$format_total" >&2
        exit 1
    fi
    format_count=$((format_count + 1))
    if [[ -z "$first_format" ]]; then
        first_format="$format"
    fi
    passed=$((passed + format_passed))
    total=$((total + format_total))
    table+=$'\n| '"$format"' | '"$format_passed"'/'"$format_total"' |'
done <<<"$format_counts"

if ((format_count == 0 || total == 0)); then
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

if ((format_count == 1)) && [[ "$first_format" == 'DNG' ]]; then
    badge_label='DNG corpus'
    badge_slug='DNG_corpus'
else
    badge_label='RAW corpus'
    badge_slug='RAW_corpus'
fi

badge="[![${badge_label}: ${passed}/${total} passed](https://img.shields.io/badge/${badge_slug}-${passed}%2F${total}-${color})](baseline.csv)"
starts="$(grep -Fxc "$start_marker" "$readme" || true)"
ends="$(grep -Fxc "$end_marker" "$readme" || true)"
if [[ "$starts" != 1 || "$ends" != 1 ]]; then
    printf '%s must contain exactly one decoder-tracker badge marker pair\n' "$readme" >&2
    exit 1
fi

tmp="$(mktemp "${readme}.XXXXXX")"
trap 'rm -f "$tmp"' EXIT
awk -v start="$start_marker" -v end="$end_marker" -v badge="$badge" -v table="$table" '
    $0 == start {
        print start
        print ""
        print badge
        print ""
        print table
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

printf 'badge: %s/%s passed across %s format(s)\n' "$passed" "$total" "$format_count"
