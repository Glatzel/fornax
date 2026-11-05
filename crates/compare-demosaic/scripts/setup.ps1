$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot/../../..
pixi install -e dev
git submodule update --init external/libraw external/raw-images

foreach ($file in @(
    "src/iop/demosaicing/amaze.cc",
    "src/iop/demosaicing/lmmse.c",
    "src/iop/demosaicing/ppg.c",
    "src/iop/demosaicing/rcd.c",
    "src/iop/demosaicing/vng.c"
))
{
    $destination = "./external/darktable/$file"

    if (Test-Path $destination) {
        continue
    }

    New-Item -ItemType Directory -Force (Split-Path $destination) | Out-Null

    $content = gh api "repos/darktable-org/darktable/contents/${file}?ref=release-5.6.2" --jq .content
    if ($LASTEXITCODE -ne 0) {
        throw "failed to download darktable/$file"
    }

    [IO.File]::WriteAllBytes(
        $destination,
        [Convert]::FromBase64String(($content -join ""))
    )
}

foreach ($file in @(
    "rtengine/ahd_demosaic_RT.cc"
    "rtengine/amaze_demosaic_RT.cc"
    "rtengine/demosaic_algos.cc"
    "rtengine/eahd_demosaic.cc"
    "rtengine/hphd_demosaic_RT.cc"
    "rtengine/lmmse_demosaic.cc"
    "rtengine/rcd_demosaic.cc"
    "rtengine/vng4_demosaic_RT.cc"
    "rtengine/iccmatrices.h"
    "rtengine/jaggedarray.h"
    "rtengine/median.h"
    "rtengine/opthelper.h"
    "rtengine/rawimagesource_i.h"
    "rtengine/rt_math.h"
))
{
    $destination = "./external/rawtherapee/$file"
    if (Test-Path $destination) {
        continue
    }

    New-Item -ItemType Directory -Force (Split-Path $destination) | Out-Null

    $content = gh api "repos/RawTherapee/RawTherapee/contents/${file}?ref=1924179107995b01b73cdc7f4fe91e44fc8227a7" --jq .content
    if ($LASTEXITCODE -ne 0) {
        throw "failed to download RawTherapee/$file"
    }

    [IO.File]::WriteAllBytes(
        $destination,
        [Convert]::FromBase64String(($content -join ""))
    )
}

if ($IsWindows)
{
    $env:Path = "$(Resolve-Path ./.pixi/envs/dev/Library/bin);$env:Path"
    $env:LIBRAW_ROOT = "$(Resolve-Path ./.pixi/envs/dev/Library)"
}
if ($IsMacOS)
{
    $env:Path = "$(Resolve-Path ./.pixi/envs/dev/lib)`:$env:Path"
    $env:DYLD_LIBRARY_PATH = "$(Resolve-Path ./.pixi/envs/dev/lib)`:$env:DYLD_LIBRARY_PATH"
    $env:LIBRAW_ROOT = "$(Resolve-Path ./.pixi/envs/dev)"
}
if ($IsLinux -and ($(uname -m) -eq 'x86_64' ))
{
    $env:Path = "$(Resolve-Path ./.pixi/envs/dev/lib)`:$env:Path"
    $env:LD_LIBRARY_PATH = "$(Resolve-Path ./.pixi/envs/dev/lib)`:$env:LD_LIBRARY_PATH"
    $env:LIBRAW_ROOT = "$(Resolve-Path ./.pixi/envs/dev)"
}
if ($IsLinux -and ($(uname -m) -eq 'aarch64' ))
{
    $env:Path = "$(Resolve-Path ./.pixi/envs/dev/lib)`:$env:Path"
    $env:LD_LIBRARY_PATH = "$(Resolve-Path ./.pixi/envs/dev/lib)`:$env:LD_LIBRARY_PATH"
    $env:LIBRAW_ROOT = "$(Resolve-Path ./.pixi/envs/dev)"
}
Set-Location $PSScriptRoot/..
