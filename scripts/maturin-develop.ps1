param (
    [ValidateSet("develop", "release")]
    $config = "develop"
)
$ROOT = git rev-parse --show-toplevel
Set-Location $PSScriptRoot/../crates/fornax-py
Remove-Item ./fornax/pyxis.pyd -ErrorAction SilentlyContinue

pixi run maturin develop --profile $config

Set-Location $ROOT
