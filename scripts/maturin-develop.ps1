& $PSScriptRoot/setup.ps1
$ROOT = git rev-parse --show-toplevel
Set-Location $PSScriptRoot/../crates/fornax-py
Remove-Item ./fornax/pyxis.pyd -ErrorAction SilentlyContinue

pixi run maturin develop

Set-Location $ROOT
