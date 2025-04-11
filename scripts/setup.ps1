$env:PYTHONPATH = Resolve-Path $PSScriptRoot/../crates/fornax-py

if ($IsWindows) {
    $env:PKG_CONFIG_PATH = Resolve-Path $PSScriptRoot/../vcpkg/installed/x64-windows-static/lib/pkgconfig
}
if ($IsLinux) {
    pixi install -e libraw
    $env:PKG_CONFIG_PATH = Resolve-Path $PSScriptRoot/../.pixi/envs/libraw/lib/pkgconfig
}