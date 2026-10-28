& $PSScriptRoot/setup.ps1
$ROOT = git rev-parse --show-toplevel
Set-Location $PSScriptRoot/../crates/fornax-py
Remove-Item ./fornax/pyxis.pyd -ErrorAction SilentlyContinue

pixi run -e python maturin develop

Set-Location $ROOT
