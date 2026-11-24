#!/usr/bin/env bash

set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

git submodule update --init external/raw-images external/libtiff-pics
git -C external/libtiff-pics lfs install --local
git -C external/libtiff-pics lfs pull
