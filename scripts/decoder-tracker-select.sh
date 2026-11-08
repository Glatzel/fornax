#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
manifest="$repo_root/crates/decoder-tracker/manifest.csv"
filter="**/*.[dD][nN][gG]"
out="$repo_root/target/decoder-tracker/selected-manifest.csv"

while (($# > 0)); do
    case "$1" in
        --manifest)
            manifest="$2"
            shift 2
            ;;
        --filter)
            filter="$2"
            shift 2
            ;;
        --out)
            out="$2"
            shift 2
            ;;
        *)
            printf 'usage: %s [--manifest FILE] [--filter GLOB] [--out FILE]\n' "$0" >&2
            exit 2
            ;;
    esac
done

sql_escape() {
    local value="$1"
    value="${value//\\//}"
    printf '%s' "${value//\'/\'\'}"
}

mkdir -p "$(dirname "$out")"
sql_path="$repo_root/crates/decoder-tracker/sql/select.sql"
sql="$(<"$sql_path")"
sql="${sql//@MANIFEST@/$(sql_escape "$manifest")}"
sql="${sql//@FILTER@/$(sql_escape "$filter")}"
sql="${sql//@OUTPUT@/$(sql_escape "$out")}"

selected="$(
    duckdb -no-agent -noheader -list :memory: -c "$sql"
)"
if [[ ! "$selected" =~ ^[0-9]+$ ]] || ((selected == 0)); then
    printf 'filters matched no corpus files\n' >&2
    exit 1
fi

printf 'selected=%s\n' "$selected"
