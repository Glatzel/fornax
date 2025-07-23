param (
    [ValidateSet("develop", "release")]
    $config = "develop"
)
$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true
$ROOT = git rev-parse --show-toplevel
&$ROOT/scripts/setup.ps1
Set-Location $PSScriptRoot

if ($config -ne 'release') {
    & "$ROOT/scripts/maturin-develop.ps1" -config $config
    & "$ROOT/scripts/pytest.ps1"
}
& "$ROOT/scripts/build-python-whl.ps1" -config $config

Set-Location $PSScriptRoot
pixi run rattler-build build
Set-Location $ROOT
