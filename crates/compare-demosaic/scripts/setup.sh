#!/usr/bin/env bash

set -euo pipefail

caller_dir="$PWD"
cd "$(dirname "${BASH_SOURCE[0]}")/../../.."

pixi install -e dev
git submodule update --init external/libraw external/raw-images

files=(
    "src/iop/demosaicing/amaze.cc"
    "src/iop/demosaicing/lmmse.c"
    "src/iop/demosaicing/ppg.c"
    "src/iop/demosaicing/rcd.c"
    "src/iop/demosaicing/vng.c"
)

for file in "${files[@]}"; do
    destination="external/darktable/$file"

    [[ -f "$destination" ]] && continue

    mkdir -p "$(dirname "$destination")"

    gh api "repos/darktable-org/darktable/contents/$file?ref=release-5.6.2" \
        --jq .content |
        base64 -d > "$destination" || exit 1
done

rawtherapee_files=(
    "rtengine/ahd_demosaic_RT.cc"
    "rtengine/amaze_demosaic_RT.cc"
    "rtengine/demosaic_algos.cc"
    "rtengine/eahd_demosaic.cc"
    "rtengine/hphd_demosaic_RT.cc"
    "rtengine/lmmse_demosaic.cc"
    "rtengine/rcd_demosaic.cc"
    "rtengine/vng4_demosaic_RT.cc"
    "rtengine/iccmatrices.h"
    "rtengine/jaggedarray.h"
    "rtengine/median.h"
    "rtengine/opthelper.h"
    "rtengine/rawimagesource_i.h"
    "rtengine/rt_math.h"
)

for file in "${rawtherapee_files[@]}"; do
    destination="external/rawtherapee/$file"

    [[ -f "$destination" ]] && continue

    mkdir -p "$(dirname "$destination")"

    gh api "repos/RawTherapee/RawTherapee/contents/$file?ref=1924179107995b01b73cdc7f4fe91e44fc8227a7" \
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

cd "$caller_dir"
