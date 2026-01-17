Set-Location $PSScriptRoot/..
pixi install
git submodule update --init --recursive
$env:CONDA_PREFIX = resolve-path $PSScriptRoot/../.pixi/envs/default
if ($IsWindows) {
    $bin = Resolve-Path $PSScriptRoot/../.pixi/envs/default/Library/bin
    $env:Path = "$bin" + ";" + "$env:Path"
    $env:PKG_CONFIG_PATH = Resolve-Path $PSScriptRoot/../.pixi/envs/default/Library/lib/pkgconfig
}
if ($IsLinux) {
    $env:PKG_CONFIG_PATH = Resolve-Path $PSScriptRoot/../.pixi/envs/default/lib/pkgconfig
}
if ($IsMacOS) {
    $env:PKG_CONFIG_PATH = Resolve-Path $PSScriptRoot/../.pixi/envs/default/lib/pkgconfig
}
