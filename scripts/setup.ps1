$env:PYTHONPATH = Resolve-Path $PSScriptRoot/../crates/fornax-py
pixi install

$env:PKG_CONFIG_PATH = Resolve-Path $PSScriptRoot/../.pixi/envs/default/libraw/x64-windows-static/lib/pkgconfig
