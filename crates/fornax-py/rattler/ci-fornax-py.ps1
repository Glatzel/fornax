param (
    [ValidateSet("develop", "release")]
    $config = "develop"
)
$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true
$ROOT = git rev-parse --show-toplevel
Set-Location $PSScriptRoot

& "$ROOT/scripts/maturin-develop.ps1" -config $config
if ($config -ne 'release') {
    & "$ROOT/scripts/pytest.ps1"
}
& "$ROOT/scripts/build-python-whl.ps1" -config $config

Set-Location $PSScriptRoot
pixi run -e pydev rattler-build build
Set-Location $ROOT
