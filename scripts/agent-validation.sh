#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

docs_only=false
fix=false
for arg in "$@"; do
    case "$arg" in
        --docs-only)
            docs_only=true
            ;;
        --fix)
            fix=true
            ;;
        *)
            printf 'usage: %s [--docs-only | --fix]\n' "$0" >&2
            exit 2
            ;;
    esac
done

if [[ "$docs_only" == true && "$fix" == true ]]; then
    printf 'usage: %s [--docs-only | --fix]\n' "$0" >&2
    exit 2
fi

clippy_flags=(
    -A clippy::too_many_arguments
    -D warnings
)
stage_timeout="${FORNAX_VALIDATION_STAGE_TIMEOUT_SECS:-1800}"

run_stage() {
    local name="$1"
    shift
    printf 'validation stage: %s\n' "$name" >&2
    if command -v timeout >/dev/null 2>&1; then
        timeout --foreground "${stage_timeout}s" "$@"
    else
        "$@"
    fi
}

if git_base="$(git merge-base HEAD origin/main 2>/dev/null)"; then
    base="$git_base"
else
    base=HEAD
fi

run_stage git-diff git diff --check "$base" --
run_stage documentation-links python3 scripts/check-doc-links.py

changed_files="$(
    git diff --name-only --diff-filter=ACMRTUXB "$base" --
    git ls-files --others --exclude-standard
)"

if [[ "$docs_only" == false ]]; then
    if [[ ! -f external/raw-images/images/colorchart-5D2-6000K.dng ]]; then
        bash scripts/setup.sh
    fi
    if [[ ! -f external/raw.pixls.us/filelist.sha256 ]]; then
        GIT_LFS_SKIP_SMUDGE=1 \
            git -c filter.lfs.required=false \
            -c submodule.external/raw.pixls.us.branch=master \
            submodule update --init --remote external/raw.pixls.us
    fi

    if [[ "$fix" == true ]]; then
        run_stage clippy-fix cargo -q clippy --workspace --all-features --all-targets --fix \
            --allow-dirty --allow-staged --exclude compare-demosaic \
            -- "${clippy_flags[@]}"
        run_stage rustfmt cargo +nightly fmt -q
    fi

    run_stage corpus-manifest cargo -q run -p decoder-tracker -- manifest --check
    run_stage golden-hashes cargo -q run --release -p decoder-tracker -- hash \
        --manifest crates/decoder-tracker/golden/test-manifest.csv \
        --check crates/decoder-tracker/golden/decode-hashes.csv
    run_stage tracker-duckdb pixi run -e tracker cargo -q test -p decoder-tracker --lib duckdb_
    run_stage workspace-tests cargo -q test --workspace --all-features --exclude compare-demosaic
    run_stage codec-feature-check cargo -q check -p fornax-codec --no-default-features
    run_stage codec-ljpeg-tests cargo -q test -p fornax-codec --features lossless-jpeg
    run_stage decoder-no-default-features \
        cargo -q test -p fornax-raw-decoder --no-default-features
    run_stage rustfmt-check cargo +nightly fmt --check -q
    RUSTDOCFLAGS="-D warnings" \
        run_stage rustdoc cargo -q doc --workspace --all-features --exclude compare-demosaic --no-deps
    run_stage workspace-clippy cargo -q clippy --workspace --all-features --all-targets \
        --exclude compare-demosaic -- "${clippy_flags[@]}"
    run_stage decoder-bench-compile cargo -q bench -p fornax-raw-decoder --no-run
fi

if grep -Eq '^crates/compare-demosaic/' <<<"$changed_files"; then
    (
        # shellcheck source=crates/compare-demosaic/scripts/setup.sh
        source ./crates/compare-demosaic/scripts/setup.sh
        cd crates/compare-demosaic

        if [[ "$fix" == true ]]; then
            run_stage compare-clippy-fix cargo -q clippy --all-features --all-targets --fix \
                --allow-dirty --allow-staged -p compare-demosaic \
                -- "${clippy_flags[@]}"
            run_stage compare-rustfmt cargo +nightly fmt -q
        fi

        RUSTDOCFLAGS="-D warnings" run_stage compare-rustdoc \
            cargo -q doc --all-features -p compare-demosaic --no-deps
        run_stage compare-clippy cargo -q clippy --all-features --all-targets -p compare-demosaic \
            -- "${clippy_flags[@]}"
        run_stage compare-tests cargo -q test --all-features -p compare-demosaic
    )
fi
