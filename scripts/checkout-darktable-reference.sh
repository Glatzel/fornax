#!/usr/bin/env bash

set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

submodule_path="external/darktable"
reference_file="src/iop/demosaicing/ppg.c"
expected_commit="$(git rev-parse "HEAD:$submodule_path")"
submodule_url="$(git config -f .gitmodules --get "submodule.$submodule_path.url")"
force=false

module_dir="$(git rev-parse --git-path "modules/$submodule_path")"
case "$module_dir" in
    /*) ;;
    *) module_dir="$repo_root/$module_dir" ;;
esac
common_git_dir="$(git rev-parse --git-common-dir)"
case "$common_git_dir" in
    /*) ;;
    *) common_git_dir="$repo_root/$common_git_dir" ;;
esac
case "$module_dir" in
    "$common_git_dir"/modules/"$submodule_path" | \
        "$common_git_dir"/worktrees/*/modules/"$submodule_path") ;;
    *)
        echo "refusing to remove unexpected darktable module path: $module_dir" >&2
        exit 1
        ;;
esac

if [[ "${1:-}" == "--force" ]]; then
    force=true
elif [[ -n "${1:-}" ]]; then
    echo "usage: $0 [--force]" >&2
    exit 2
fi

if $force; then
    git submodule deinit -f -- "$submodule_path" >/dev/null 2>&1 || true
    rm -rf -- "$repo_root/$submodule_path" "$module_dir"
fi

if [[ -e "$submodule_path/.git" ]]; then
    actual_commit="$(git -C "$submodule_path" rev-parse HEAD)"
    if [[ "$actual_commit" != "$expected_commit" ]]; then
        echo "darktable is at $actual_commit, expected $expected_commit" >&2
        echo "rerun with --force to replace the checkout" >&2
        exit 1
    fi
    if [[ "$(git -C "$submodule_path" config --bool remote.origin.promisor 2>/dev/null || true)" != "true" ]]; then
        echo "darktable is a full clone; rerun with --force to reclaim its object store" >&2
    fi
elif [[ -d "$submodule_path" && -n "$(ls -A "$submodule_path")" ]]; then
    echo "darktable directory is populated but is not a Git checkout" >&2
    echo "rerun with --force to replace it" >&2
    exit 1
fi

if [[ ! -e "$submodule_path/.git" ]]; then
    rm -rf -- "$module_dir"
    git submodule init -- "$submodule_path"
    mkdir -p "$(dirname "$submodule_path")"
    git clone --filter=blob:none --no-checkout --depth 1 --single-branch \
        "$submodule_url" "$submodule_path"
    git -C "$submodule_path" fetch --depth 1 origin "$expected_commit"
fi

# A clone created with --no-checkout initially keeps its Git directory inline.
# Reclaim any metadata left by a previous submodule checkout before absorbing it.
if [[ -d "$submodule_path/.git" && -d "$module_dir" ]]; then
    rm -rf -- "$module_dir"
fi

git -C "$submodule_path" sparse-checkout init --no-cone
git -C "$submodule_path" sparse-checkout set --no-cone "/$reference_file"
git -C "$submodule_path" checkout --detach "$expected_commit"
git submodule absorbgitdirs -- "$submodule_path"

test -f "$submodule_path/$reference_file"
printf 'darktable reference checkout: %s\n' "$expected_commit"
du -sh "$submodule_path"
