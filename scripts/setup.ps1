$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot/..
pixi install -e dev
git submodule update --init external/libraw external/raw-images

$darktablePath = "external/darktable"
$darktableCommit = (& git rev-parse "HEAD:$darktablePath").Trim()
if (-not (Test-Path -LiteralPath "$darktablePath/.git")) {
    Remove-Item -LiteralPath $darktablePath -Recurse -Force -ErrorAction SilentlyContinue
    gh repo clone darktable-org/darktable $darktablePath -- --depth=1 --filter=blob:none --sparse
}
git -C $darktablePath fetch --depth=1 origin $darktableCommit
git -C $darktablePath sparse-checkout set --no-cone "/src/iop/demosaicing/ppg.c"
git -C $darktablePath checkout --detach $darktableCommit


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
