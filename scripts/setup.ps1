Set-Location $PSScriptRoot/..
pixi install
git submodule update --init --recursive
$env:CONDA_PREFIX = resolve-path $PSScriptRoot/../.pixi/envs/default
if ($IsWindows) {
    $bin = Resolve-Path $PSScriptRoot/../.pixi/envs/default/Library/bin
    $env:Path = "$bin" + ";" + "$env:Path"
    $env:PKG_CONFIG_PATH = Resolve-Path $PSScriptRoot/../.pixi/envs/default/libraw/x64-windows-static/lib/pkgconfig
}
if ($IsLinux) {
    sudo apt-get update
    sudo apt-get install libgomp1
    $env:PKG_CONFIG_PATH = Resolve-Path $PSScriptRoot/../.pixi/envs/default/libraw/x64-linux-release/lib/pkgconfig
}
if ($IsMacOS) {
    $env:PKG_CONFIG_PATH = Resolve-Path $PSScriptRoot/../.pixi/envs/default/libraw/arm64-osx-release/lib/pkgconfig
    $env:MACOSX_DEPLOYMENT_TARGET="14.0"
}
