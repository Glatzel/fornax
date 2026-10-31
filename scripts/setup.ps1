$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot/..
pixi install -e dev
git submodule update --init external/libraw external/raw-images

foreach ($file in  @(
        "src/iop/demosaicing/amaze.cc",
        "src/iop/demosaicing/lmmse.c",
        "src/iop/demosaicing/ppg.c",
        "src/iop/demosaicing/rcd.c",
        "src/iop/demosaicing/vng.c"
    ))
{
    $destination = Join-Path "external/darktable" $file

    New-Item -ItemType Directory -Force (Split-Path $destination) | Out-Null

    $content = gh api "repos/darktable-org/darktable/contents/${file}?ref=release-5.6.2" --jq .content
    if ($LASTEXITCODE -ne 0)
    {
        throw "failed to download darktable/$file"
    }

    [IO.File]::WriteAllBytes(
        $destination,
        [Convert]::FromBase64String(($content -join ""))
    )
}

if ($IsWindows)
{
    $env:Path = "$(Resolve-Path $PSScriptRoot/../.pixi/envs/dev/Library/bin);$env:Path"
    $env:LIBRAW_ROOT = "$(Resolve-Path $PSScriptRoot/../.pixi/envs/dev/Library)"
}
if ($IsMacOS)
{
    $env:Path = "$(Resolve-Path $PSScriptRoot/../.pixi/envs/dev/lib)`:$env:Path"
    $env:DYLD_LIBRARY_PATH = "$(Resolve-Path $PSScriptRoot/../.pixi/envs/dev/lib)`:$env:DYLD_LIBRARY_PATH"
    $env:LIBRAW_ROOT = "$(Resolve-Path $PSScriptRoot/../.pixi/envs/dev)"
}
if ($IsLinux -and ($(uname -m) -eq 'x86_64' ))
{
    $env:Path = "$(Resolve-Path $PSScriptRoot/../.pixi/envs/dev/lib)`:$env:Path"
    $env:LD_LIBRARY_PATH = "$(Resolve-Path $PSScriptRoot/../.pixi/envs/dev/lib)`:$env:LD_LIBRARY_PATH"
    $env:LIBRAW_ROOT = "$(Resolve-Path $PSScriptRoot/../.pixi/envs/dev)"
}
if ($IsLinux -and ($(uname -m) -eq 'aarch64' ))
{
    $env:Path = "$(Resolve-Path $PSScriptRoot/../.pixi/envs/dev/lib)`:$env:Path"
    $env:LD_LIBRARY_PATH = "$(Resolve-Path $PSScriptRoot/../.pixi/envs/dev/lib)`:$env:LD_LIBRARY_PATH"
    $env:LIBRAW_ROOT = "$(Resolve-Path $PSScriptRoot/../.pixi/envs/dev)"
}
