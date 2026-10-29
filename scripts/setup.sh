#!/usr/bin/env bash

set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

pixi install -e dev
git submodule update --init external/libraw external/raw-images

darktable_path="external/darktable"
darktable_commit="$(git rev-parse "HEAD:$darktable_path")"
if [[ ! -e "$darktable_path/.git" ]]; then
    rm -rf -- "$darktable_path"
    gh repo clone darktable-org/darktable "$darktable_path" -- \
        --depth=1 --filter=blob:none --sparse
fi
git -C "$darktable_path" fetch --depth=1 origin "$darktable_commit"
git -C "$darktable_path" sparse-checkout set --no-cone \
    "/src/iop/demosaicing/ppg.c"
git -C "$darktable_path" checkout --detach "$darktable_commit"

pixi_env_dir="$(realpath .pixi/envs/dev)"

if [[ "$(uname -s)" == "Darwin" ]]; then
    export PATH="$pixi_env_dir/lib:$PATH"
    export DYLD_LIBRARY_PATH="$pixi_env_dir/lib${DYLD_LIBRARY_PATH:+:$DYLD_LIBRARY_PATH}"
    export LIBRAW_ROOT="$pixi_env_dir"
elif [[ "$(uname -s)" == "Linux" ]]; then
    export PATH="$pixi_env_dir/lib:$PATH"
    export LD_LIBRARY_PATH="$pixi_env_dir/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
    export LIBRAW_ROOT="$pixi_env_dir"
fi
