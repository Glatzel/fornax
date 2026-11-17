#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
runner="$repo_root/scripts/agent-run.sh"

if [[ ! -f "$runner" ]]; then
    printf 'missing agent runner: %s\n' "$runner" >&2
    exit 1
fi

test_root="$(mktemp -d)"
trap 'rm -rf "$test_root"' EXIT

export FORNAX_AGENT_LOG_DIR="$test_root/logs"

success_output="$test_root/success.out"
"$runner" --label success --tail 5 -- bash -c \
    'for line in $(seq 1 20); do printf "line-%s\n" "$line"; done' \
    >"$success_output"

grep -Fq 'status: 0' "$success_output"
grep -Fq 'line-20' "$success_output"
if grep -Fq 'line-15' "$success_output"; then
    printf 'success summary exceeded requested tail\n' >&2
    exit 1
fi

success_log="$(sed -n 's/^log: //p' "$success_output")"
if [[ -z "$success_log" || ! -f "$success_log" ]]; then
    printf 'runner did not report an existing log file\n' >&2
    exit 1
fi
if ! grep -Fq 'line-1' "$success_log"; then
    printf 'full command output was not captured\n' >&2
    exit 1
fi

failure_output="$test_root/failure.out"
set +e
"$runner" --label failure --tail 10 -- bash -c \
    'printf "before failure\n"; printf "boom\n" >&2; exit 7' \
    >"$failure_output"
failure_status=$?
set -e

if [[ "$failure_status" -ne 7 ]]; then
    printf 'runner changed failure status to %s\n' "$failure_status" >&2
    exit 1
fi
grep -Fq 'status: 7' "$failure_output"
grep -Fq 'boom' "$failure_output"

printf 'agent runner test: ok\n'
