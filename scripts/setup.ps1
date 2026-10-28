Set-Location $PSScriptRoot/..
pixi install -e dev
git submodule update --init external/libraw external/raw-images
if ($IsWindows) {
    $env:Path = "$(Resolve-Path $PSScriptRoot/../.pixi/envs/dev/Library/bin);$env:Path"
    $env:LIBRAW_ROOT = "$(Resolve-Path $PSScriptRoot/../.pixi/envs/dev/Library)"
}
if ($IsMacOS) {
    $env:Path = "$(Resolve-Path $PSScriptRoot/../.pixi/envs/dev/lib)`:$env:Path"
    $env:DYLD_LIBRARY_PATH = "$(Resolve-Path $PSScriptRoot/../.pixi/envs/dev/lib)`:$env:DYLD_LIBRARY_PATH"
    $env:LIBRAW_ROOT = "$(Resolve-Path $PSScriptRoot/../.pixi/envs/dev)"
}
if ($IsLinux -and ($(uname -m) -eq 'x86_64' )) {
    $env:Path = "$(Resolve-Path $PSScriptRoot/../.pixi/envs/dev/lib)`:$env:Path"
    $env:LD_LIBRARY_PATH = "$(Resolve-Path $PSScriptRoot/../.pixi/envs/dev/lib)`:$env:LD_LIBRARY_PATH"
    $env:LIBRAW_ROOT = "$(Resolve-Path $PSScriptRoot/../.pixi/envs/dev)"
}
if ($IsLinux -and ($(uname -m) -eq 'aarch64' )) {
    $env:Path = "$(Resolve-Path $PSScriptRoot/../.pixi/envs/dev/lib)`:$env:Path"
    $env:LD_LIBRARY_PATH = "$(Resolve-Path $PSScriptRoot/../.pixi/envs/dev/lib)`:$env:LD_LIBRARY_PATH"
    $env:LIBRAW_ROOT = "$(Resolve-Path $PSScriptRoot/../.pixi/envs/dev)"
}
