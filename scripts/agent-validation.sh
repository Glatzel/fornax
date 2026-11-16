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

if git_base="$(git merge-base HEAD origin/main 2>/dev/null)"; then
    base="$git_base"
else
    base=HEAD
fi

git diff --check "$base" --
python3 scripts/check-doc-links.py

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
        cargo -q clippy --workspace --all-features --all-targets --fix \
            --allow-dirty --allow-staged --exclude compare-demosaic \
            -- "${clippy_flags[@]}"
        cargo +nightly fmt -q
    fi

    cargo -q run -p decoder-tracker -- manifest --check
    cargo -q run --release -p decoder-tracker -- hash \
        --manifest crates/decoder-tracker/golden/selected-manifest.csv \
        --check crates/decoder-tracker/golden/decode-hashes.csv
    pixi run -e tracker cargo -q test -p decoder-tracker --lib duckdb_
    cargo -q test --workspace --all-features --exclude compare-demosaic
    cargo -q check -p fornax-codec --no-default-features
    cargo -q test -p fornax-codec --features lossless-jpeg
    cargo -q test \
        -p fornax-raw-sony \
        -p fornax-raw-canon \
        -p fornax-raw-nikon \
        -p fornax-raw-fujifilm \
        -p fornax-raw-hasselblad \
        -p fornax-raw-panasonic \
        -p fornax-raw-phase-one
    cargo +nightly fmt --check -q
    RUSTDOCFLAGS="-D warnings" \
        cargo -q doc --workspace --all-features --exclude compare-demosaic --no-deps
    cargo -q clippy --workspace --all-features --all-targets \
        --exclude compare-demosaic -- "${clippy_flags[@]}"
    cargo -q bench -p fornax-decoder --no-run
fi

if grep -Eq '^crates/compare-demosaic/' <<<"$changed_files"; then
    (
        # shellcheck source=crates/compare-demosaic/scripts/setup.sh
        source ./crates/compare-demosaic/scripts/setup.sh
        cd crates/compare-demosaic

        if [[ "$fix" == true ]]; then
            cargo -q clippy --all-features --all-targets --fix \
                --allow-dirty --allow-staged -p compare-demosaic \
                -- "${clippy_flags[@]}"
            cargo +nightly fmt -q
        fi

        RUSTDOCFLAGS="-D warnings" cargo -q doc --all-features -p compare-demosaic --no-deps
        cargo -q clippy --all-features --all-targets -p compare-demosaic \
            -- "${clippy_flags[@]}"
        cargo -q test --all-features -p compare-demosaic
    )
fi
