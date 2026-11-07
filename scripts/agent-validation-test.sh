#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
validation_script="$repo_root/scripts/agent-validation.sh"

if [[ ! -f "$validation_script" ]]; then
    printf 'missing validation script: %s\n' "$validation_script" >&2
    exit 1
fi

test_root="$(mktemp -d)"
trap 'rm -rf "$test_root"' EXIT

mkdir -p "$test_root/scripts" "$test_root/bin" "$test_root/crates/compare-demosaic/scripts"
cp "$validation_script" "$test_root/scripts/agent-validation.sh"

log_file="$test_root/commands.log"
export VALIDATION_TEST_LOG="$log_file"
export VALIDATION_TEST_CHANGED_FILES

cat >"$test_root/bin/cargo" <<'EOF'
#!/usr/bin/env bash
printf 'cargo:%s\n' "$*" >>"$VALIDATION_TEST_LOG"
EOF

cat >"$test_root/bin/git" <<'EOF'
#!/usr/bin/env bash
printf 'git:%s\n' "$*" >>"$VALIDATION_TEST_LOG"

if [[ "$*" == "merge-base HEAD origin/main" ]]; then
    printf 'base\n'
elif [[ "$*" == "diff --name-only --diff-filter=ACMRTUXB base --" ]]; then
    printf '%b' "$VALIDATION_TEST_CHANGED_FILES"
fi
EOF

cat >"$test_root/scripts/setup.sh" <<'EOF'
#!/usr/bin/env bash
printf 'setup:root\n' >>"$VALIDATION_TEST_LOG"
EOF

cat >"$test_root/crates/compare-demosaic/scripts/setup.sh" <<'EOF'
#!/usr/bin/env bash
printf 'setup:compare\n' >>"$VALIDATION_TEST_LOG"
EOF

chmod +x \
    "$test_root/bin/cargo" \
    "$test_root/bin/git" \
    "$test_root/scripts/setup.sh" \
    "$test_root/scripts/agent-validation.sh"

run_validation() {
    changed_files="$1"
    shift
    VALIDATION_TEST_CHANGED_FILES="$changed_files" PATH="$test_root/bin:$PATH" \
        "$test_root/scripts/agent-validation.sh" "$@"
}

assert_logged() {
    if ! grep -Fxq "$1" "$log_file"; then
        printf 'expected command was not logged: %s\n' "$1" >&2
        printf 'commands:\n' >&2
        sed 's/^/  /' "$log_file" >&2
        exit 1
    fi
}

assert_logged_prefix() {
    if ! grep -Fq "$1" "$log_file"; then
        printf 'expected command was not logged: %s\n' "$1" >&2
        printf 'commands:\n' >&2
        sed 's/^/  /' "$log_file" >&2
        exit 1
    fi
}

: >"$log_file"
run_validation 'AGENTS.md\n'

assert_logged 'setup:root'
assert_logged 'cargo:-q test --workspace --all-features --exclude compare-demosaic'
assert_logged 'cargo:+nightly fmt --check -q'
assert_logged 'cargo:-q doc --workspace --all-features --exclude compare-demosaic --no-deps'
assert_logged_prefix \
    'cargo:-q clippy --workspace --all-features --all-targets --exclude compare-demosaic -- '
assert_logged 'cargo:-q bench -p bench-decoder --no-run'
assert_logged 'git:diff --check base --'

if grep -Fq 'compare-demosaic' "$log_file" |
    grep -Fv -- '--exclude compare-demosaic'; then
    printf 'unexpected compare-demosaic validation\n' >&2
    exit 1
fi

: >"$log_file"
run_validation 'AGENTS.md\ncrates/compare-demosaic/src/lib.rs\n'

assert_logged 'setup:compare'
assert_logged 'cargo:-q doc --all-features -p compare-demosaic --no-deps'
assert_logged_prefix \
    'cargo:-q clippy --all-features --all-targets -p compare-demosaic -- '
assert_logged 'cargo:-q test --all-features -p compare-demosaic'

mkdir -p "$test_root/external/raw-images/images"
touch "$test_root/external/raw-images/images/colorchart-5D2-6000K.dng"

: >"$log_file"
run_validation 'scripts/agent-validation.sh\n'

assert_logged 'cargo:-q test --workspace --all-features --exclude compare-demosaic'

if grep -Fxq 'setup:root' "$log_file"; then
    printf 'validation reran setup with an existing raw-image fixture\n' >&2
    exit 1
fi

: >"$log_file"
run_validation 'AGENTS.md\ncrates/compare-demosaic/src/lib.rs\n' --fix

assert_logged_prefix \
    'cargo:-q clippy --workspace --all-features --all-targets --fix --allow-dirty --allow-staged --exclude compare-demosaic -- '
assert_logged_prefix \
    'cargo:-q clippy --all-features --all-targets --fix --allow-dirty --allow-staged -p compare-demosaic -- '
assert_logged 'cargo:+nightly fmt -q'
assert_logged 'cargo:-q test --workspace --all-features --exclude compare-demosaic'
assert_logged 'cargo:+nightly fmt --check -q'
assert_logged_prefix \
    'cargo:-q clippy --workspace --all-features --all-targets --exclude compare-demosaic -- '
assert_logged 'cargo:-q test --all-features -p compare-demosaic'

: >"$log_file"
run_validation 'AGENTS.md\n' --docs-only

assert_logged 'git:diff --check base --'

if grep -Eq '^(cargo|setup):' "$log_file"; then
    printf 'docs-only validation ran Rust or setup commands\n' >&2
    exit 1
fi

printf 'agent validation test: ok\n'
