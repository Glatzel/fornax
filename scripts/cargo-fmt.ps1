$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true
$ROOT = git rev-parse --show-toplevel
Set-Location $PSScriptRoot/..
& $PSScriptRoot/setup.ps1
if ($env:CI) {
    pixi run cargo +nightly fmt --all -- --check
}
else {
    pixi run cargo +nightly fmt --all
}

Set-Location $ROOT
