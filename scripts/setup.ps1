$env:PYTHONPATH = Resolve-Path $PSScriptRoot/../crates/fornax-py
pixi install -e libraw
if ($IsWindows) {
    $env:PKG_CONFIG_PATH = Resolve-Path $PSScriptRoot/../Library/lib/pkgconfig
    # $env:PKG_CONFIG_PATH = Resolve-Path $PSScriptRoot/../vcpkg/installed/x64-windows-static/lib/pkgconfig
}
if ($IsLinux) {
    $env:PKG_CONFIG_PATH = Resolve-Path $PSScriptRoot/../.pixi/envs/libraw/lib/pkgconfig
}