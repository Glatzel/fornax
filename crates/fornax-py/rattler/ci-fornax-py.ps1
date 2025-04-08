param (
    [ValidateSet("develop","release")]
    $config = "develop"
)
$ROOT = git rev-parse --show-toplevel
Set-Location $PSScriptRoot

& "$ROOT/scripts/maturin-develop.ps1" -config $config
& "$ROOT/scripts/pytest.ps1"
& "$ROOT/scripts/build-python-whl.ps1" -config $config

Set-Location $PSScriptRoot
pixi run -e pydev rattler-build build
Set-Location $ROOT
