#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
selected=""
baseline="$repo_root/crates/decoder-tracker/data/baseline.csv"
corpus_manifest="$repo_root/crates/decoder-tracker/data/manifest.csv"
work_dir="$repo_root/target/decoder-tracker/update"
github_output=""
shards=()
tracker="$repo_root/crates/decoder-tracker/scripts/tracker_data.py"
if [[ -n "${DECODER_TRACKER_PYTHON:-}" ]]; then
    python_bin="$DECODER_TRACKER_PYTHON"
elif [[ -x "$repo_root/.pixi/envs/tracker/bin/python" ]]; then
    python_bin="$repo_root/.pixi/envs/tracker/bin/python"
else
    python_bin="$repo_root/.pixi/envs/dev/bin/python"
fi

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
        --corpus-manifest)
            corpus_manifest="$2"
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
            printf 'usage: %s --selected FILE [--baseline FILE] [--corpus-manifest FILE] [--work-dir DIR] -- SHARD...\n' "$0" >&2
            exit 2
            ;;
    esac
done

if [[ -z "$selected" || ${#shards[@]} -eq 0 ]]; then
    printf 'selected manifest and at least one shard are required\n' >&2
    exit 2
fi

mkdir -p "$work_dir"
rm -f "$work_dir/candidate.csv" "$work_dir/diff.csv" "$work_dir/baseline.csv"
candidate="$work_dir/candidate.csv"
"$python_bin" "$tracker" merge \
    --manifest "$selected" \
    --out "$candidate" \
    "${shards[@]}"

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

baseline_header="$(head -n 1 "$baseline")"
legacy_header='status,camera,format,path,decode_pass,raw_mosaic_pass,raw_sha256,process_metadata_pass,handoff_pass,error_kind,width,height,cfa'
if [[ "$baseline_header" == "$legacy_header" ]]; then
    output="$work_dir/legacy-baseline.csv"
    "$python_bin" "$tracker" migrate \
        --baseline "$baseline" \
        --selected "$selected" \
        --corpus-manifest "$corpus_manifest" \
        --out "$output"
    baseline="$output"
fi

output="$work_dir/diff.csv"
"$python_bin" "$tracker" diff \
    --selected "$selected" \
    --out "$output" \
    --allow-changes \
    "$baseline" "$candidate"

if [[ "$(wc -l <"$output")" -le 1 ]]; then
    rm -f "$output"
    emit_output "outcome=unchanged"
    emit_output "changed=false"
    summarize unchanged
    exit 0
fi

output="$work_dir/baseline.csv"
"$python_bin" "$tracker" update \
    --baseline "$baseline" \
    --candidate "$candidate" \
    --out "$output"
emit_output "outcome=changed"
emit_output "changed=true"
summarize changed
