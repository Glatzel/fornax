pixi install
$env:PYTHONPATH = Resolve-Path $PSScriptRoot/../crates/fornax-py

if ($IsWindows) {
    $pkg_config_exe = Resolve-Path $PSScriptRoot/../.pixi/envs/default/Library/bin
    $env:Path = "$pkg_config_exe;$env:Path"
    $env:PKG_CONFIG_PATH = Resolve-Path $PSScriptRoot/../.pixi/envs/default/libraw/x64-windows-static/lib/pkgconfig
}
if ($IsLinux) {
    $pkg_config_exe = Resolve-Path $PSScriptRoot/../.pixi/envs/default/bin
    $env:Path = "$pkg_config_exe" + ":" + "$env:Path"
    $env:PKG_CONFIG_PATH = Resolve-Path $PSScriptRoot/../.pixi/envs/default/libraw/x64-linux-release/lib/pkgconfig
}
