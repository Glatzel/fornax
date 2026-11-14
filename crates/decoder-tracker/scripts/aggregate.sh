#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
selected=""
baseline="$repo_root/crates/decoder-tracker/data/baseline.csv"
work_dir="$repo_root/target/decoder-tracker/update"
github_output=""
shards=()

while (($# > 0)); do
    case "$1" in
        --selected)
            selected="$2"
            shift 2
            ;;
        --baseline)
            baseline="$2"
            shift 2
            ;;
        --work-dir)
            work_dir="$2"
            shift 2
            ;;
        --github-output)
            github_output="$2"
            shift 2
            ;;
        --)
            shift
            shards+=("$@")
            break
            ;;
        *)
            printf 'usage: %s --selected FILE [--baseline FILE] [--work-dir DIR] -- SHARD...\n' "$0" >&2
            exit 2
            ;;
    esac
done

if [[ -z "$selected" || ${#shards[@]} -eq 0 ]]; then
    printf 'selected manifest and at least one shard are required\n' >&2
    exit 2
fi

sql_escape() {
    local value="$1"
    value="${value//\\//}"
    printf '%s' "${value//\'/\'\'}"
}

render_sql() {
    local template="$1"
    local sql
    sql="$(<"$repo_root/crates/decoder-tracker/sql/$template")"
    sql="${sql//@MANIFEST@/$(sql_escape "$selected")}"
    sql="${sql//@SELECTED@/$(sql_escape "$selected")}"
    sql="${sql//@BASELINE@/$(sql_escape "$baseline")}"
    sql="${sql//@CANDIDATE@/$(sql_escape "$candidate")}"
    sql="${sql//@OUTPUT@/$(sql_escape "$output")}"
    sql="${sql//@SHARDS@/$shard_sql}"
    printf '%s' "$sql"
}

expected_header='status,camera,format,path,decode_pass,raw_mosaic_pass,raw_sha256,process_metadata_pass,handoff_pass,error_kind,width,height,cfa'
for shard in "${shards[@]}"; do
    header="$(head -n 1 "$shard")"
    if [[ "$header" != "$expected_header" ]]; then
        printf 'shard schema mismatch in %s\n' "$shard" >&2
        exit 1
    fi
done

mkdir -p "$work_dir"
rm -f "$work_dir/candidate.csv" "$work_dir/diff.csv" "$work_dir/baseline.csv"
candidate="$work_dir/candidate.csv"
output="$candidate"
shard_sql=""
for shard in "${shards[@]}"; do
    if [[ -n "$shard_sql" ]]; then
        shard_sql+=", "
    fi
    shard_sql+="'$(sql_escape "$shard")'"
done

merge_values="$(
    duckdb -no-agent -csv -noheader :memory: -c "$(render_sql merge.sql)"
)"
IFS=, read -r duplicates missing unexpected rows_merged invalid_values invalid_order invalid_error invalid_status <<<"$merge_values"
if ((duplicates > 0)); then
    printf 'merged shards contain %s duplicate rows\n' "$duplicates" >&2
    exit 1
fi
if ((missing > 0 || unexpected > 0)); then
    printf 'merged shards are incomplete: %s missing and %s unexpected rows\n' "$missing" "$unexpected" >&2
    exit 1
fi
if ((invalid_values > 0 || invalid_order > 0 || invalid_error > 0 || invalid_status > 0)); then
    printf 'shard validation failed: stages=%s order=%s errors=%s status=%s\n' \
        "$invalid_values" "$invalid_order" "$invalid_error" "$invalid_status" >&2
    exit 1
fi

emit_output() {
    if [[ -n "$github_output" ]]; then
        printf '%s\n' "$1" >>"$github_output"
    fi
    printf '%s\n' "$1"
}

summarize() {
    if [[ -n "${GITHUB_STEP_SUMMARY:-}" ]]; then
        printf 'Aggregate outcome: %s\n' "$1" >>"$GITHUB_STEP_SUMMARY"
    fi
}

if [[ ! -f "$baseline" ]]; then
    cp "$candidate" "$work_dir/baseline.csv"
    emit_output "outcome=bootstrap"
    emit_output "changed=true"
    summarize bootstrap
    exit 0
fi

output="$work_dir/diff.csv"
diff_values="$(
    duckdb -no-agent -csv -noheader :memory: -c "$(render_sql diff.sql)"
)"
IFS=, read -r missing extra changed <<<"$diff_values"
if ((missing == 0 && extra == 0 && changed == 0)); then
    rm -f "$output"
    emit_output "outcome=unchanged"
    emit_output "changed=false"
    summarize unchanged
    exit 0
fi

output="$work_dir/baseline.csv"
duckdb -no-agent -csv -noheader :memory: -c "$(render_sql upsert.sql)" >/dev/null
emit_output "outcome=changed"
emit_output "changed=true"
summarize changed
