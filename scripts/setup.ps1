$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot/..
pixi install -e dev
git submodule update --init external/libraw external/raw-images

$darktablePath = "external/darktable"
$darktableCommit = (& git rev-parse "HEAD:$darktablePath").Trim()
if ($LASTEXITCODE -ne 0) {
    throw "failed to resolve the pinned darktable commit"
}
if (-not (Test-Path -LiteralPath "$darktablePath/.git")) {
    Remove-Item -LiteralPath $darktablePath -Recurse -Force -ErrorAction SilentlyContinue
    gh repo clone darktable-org/darktable $darktablePath -- --depth=1 --filter=blob:none --sparse
    if ($LASTEXITCODE -ne 0) {
        throw "failed to clone darktable"
    }
}
git -C $darktablePath fetch --depth=1 origin $darktableCommit
if ($LASTEXITCODE -ne 0) {
    throw "failed to fetch darktable commit $darktableCommit"
}
git -C $darktablePath sparse-checkout set --no-cone "/src/iop/demosaicing/ppg.c"
if ($LASTEXITCODE -ne 0) {
    throw "failed to configure darktable sparse checkout"
}
git -C $darktablePath checkout --detach $darktableCommit
if ($LASTEXITCODE -ne 0) {
    throw "failed to checkout darktable commit $darktableCommit"
}

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
