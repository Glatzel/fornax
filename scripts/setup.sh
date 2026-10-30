#!/usr/bin/env bash

set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

pixi install -e dev
git submodule update --init external/libraw external/raw-images

files=(
    "src/iop/demosaicing/ppg.c"
    "src/iop/demosaicing/rcd.c"
)
for file in "${files[@]}"; do
    destination="external/darktable/$file"
    mkdir -p "$(dirname "$destination")"

    gh api "repos/darktable-org/darktable/contents/$file?ref=release-5.6.2" \
        --jq .content |
        base64 -d > "$destination" || exit 1
done

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
