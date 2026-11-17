#!/usr/bin/env bash

set -euo pipefail

usage() {
    cat >&2 <<'EOF'
usage: scripts/agent-run.sh [--label NAME] [--tail LINES] [--] COMMAND...

Run a noisy command with stdout and stderr captured to a log. Print the log
path, exit status, and a bounded tail. FORNAX_AGENT_LOG_DIR selects another log
directory.
EOF
}

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
label="run"
tail_lines=80

while (($# > 0)); do
    case "$1" in
        --label)
            if (($# < 2)); then
                usage
                exit 2
            fi
            label="$2"
            shift 2
            ;;
        --tail)
            if (($# < 2)); then
                usage
                exit 2
            fi
            tail_lines="$2"
            shift 2
            ;;
        --help | -h)
            usage
            exit 0
            ;;
        --)
            shift
            break
            ;;
        -*)
            usage
            exit 2
            ;;
        *)
            break
            ;;
    esac
done

if (($# == 0)); then
    usage
    exit 2
fi
if [[ ! "$tail_lines" =~ ^[1-9][0-9]*$ ]]; then
    printf 'agent-run: --tail must be a positive integer\n' >&2
    exit 2
fi

safe_label="$(printf '%s' "$label" | tr -c '[:alnum:]_.-' '-')"
safe_label="${safe_label:-run}"
log_dir="${FORNAX_AGENT_LOG_DIR:-$repo_root/target/agent-logs}"
mkdir -p "$log_dir"
log_file="$log_dir/$safe_label-$(date -u +%Y%m%dT%H%M%SZ)-$$.log"

printf 'command:'
printf ' %q' "$@"
printf '\nlog: %s\n' "$log_file"

SECONDS=0
set +e
"$@" >"$log_file" 2>&1
status=$?
set -e

printf 'status: %s\nduration: %ss\ntail (last %s lines):\n' \
    "$status" "$SECONDS" "$tail_lines"
tail -n "$tail_lines" "$log_file"
exit "$status"
