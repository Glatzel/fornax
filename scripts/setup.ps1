$env:PYTHONPATH = Resolve-Path $PSScriptRoot/../crates/fornax-py
pixi install -e libraw
if ($IsWindows) {
    $env:PKG_CONFIG_PATH = Resolve-Path $PSScriptRoot/../.pixi/envs/libraw/Library/lib/pkgconfig
}
if ($IsLinux) {
    $env:PKG_CONFIG_PATH = Resolve-Path $PSScriptRoot/../.pixi/envs/libraw/lib/pkgconfig
}