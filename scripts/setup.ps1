$env:PKG_CONFIG_PATH = Resolve-Path $PSScriptRoot/../vcpkg/installed/x64-windows-static/lib/pkgconfig
$env:PYTHONPATH = Resolve-Path $PSScriptRoot/../crates/fornax-py
