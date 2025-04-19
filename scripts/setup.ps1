pixi install
$env:PYTHONPATH = Resolve-Path $PSScriptRoot/../crates/fornax-py
$pkg_config_exe = Resolve-Path $PSScriptRoot/../.pixi/envs/default/Library/bin
$env:Path = "$pkg_config_exe;$env:Path"
$env:PKG_CONFIG_PATH = Resolve-Path $PSScriptRoot/../.pixi/envs/default/libraw/x64-windows-static/lib/pkgconfig
