#!/usr/bin/env bash

set -e

cd "$(dirname "${BASH_SOURCE[0]}")/.."

pixi install
git submodule update --init external/libraw external/raw-images

export CONDA_PREFIX="$(realpath .pixi/envs/default)"

if [[ "$(uname -s)" == "Darwin" ]]; then
    export PATH="$CONDA_PREFIX/lib:$PATH"
    export DYLD_LIBRARY_PATH="$CONDA_PREFIX/lib${DYLD_LIBRARY_PATH:+:$DYLD_LIBRARY_PATH}"
    export LIBRAW_ROOT="$CONDA_PREFIX"
elif [[ "$(uname -s)" == "Linux" ]]; then
    export PATH="$CONDA_PREFIX/lib:$PATH"
    export LD_LIBRARY_PATH="$CONDA_PREFIX/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
    export LIBRAW_ROOT="$CONDA_PREFIX"
fi
