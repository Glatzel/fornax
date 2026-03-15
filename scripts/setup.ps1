Set-Location $PSScriptRoot/..
if ((-not $IsLinux) -and $env:CI_CARGO_TEST) {
    & $PSScriptRoot/install-dnc.ps1
    git submodule update --init --recursive
}
pixi install

$env:CONDA_PREFIX = resolve-path $PSScriptRoot/../.pixi/envs/default
if ($IsWindows) {
    $env:Path = "$(Resolve-Path $PSScriptRoot/../.pixi/envs/default/Library/bin);$env:Path"
    $env:LIBRAW_ROOT = "$(Resolve-Path $PSScriptRoot/../.pixi/envs/default/Library)"
}
if ($IsMacOS) {
    $env:Path = "$(Resolve-Path $PSScriptRoot/../.pixi/envs/default/lib)`:$env:Path"
    $env:DYLD_LIBRARY_PATH = "$(Resolve-Path $PSScriptRoot/../.pixi/envs/default/lib)`:$env:DYLD_LIBRARY_PATH"
    $env:LIBRAW_ROOT = "$(Resolve-Path $PSScriptRoot/../.pixi/envs/default)"
}
if ($IsLinux -and ($(uname -m) -eq 'x86_64' )) {
    $env:Path = "$(Resolve-Path $PSScriptRoot/../.pixi/envs/default/lib)`:$env:Path"
    $env:LD_LIBRARY_PATH = "$(Resolve-Path $PSScriptRoot/../.pixi/envs/default/lib)`:$env:LD_LIBRARY_PATH"
    $env:LIBRAW_ROOT = "$(Resolve-Path $PSScriptRoot/../.pixi/envs/default)"
}
if ($IsLinux -and ($(uname -m) -eq 'aarch64' )) {
    $env:Path = "$(Resolve-Path $PSScriptRoot/../.pixi/envs/default/lib)`:$env:Path"
    $env:LD_LIBRARY_PATH = "$(Resolve-Path $PSScriptRoot/../.pixi/envs/default/lib)`:$env:LD_LIBRARY_PATH"
    $env:LIBRAW_ROOT = "$(Resolve-Path $PSScriptRoot/../.pixi/envs/default)"
}
