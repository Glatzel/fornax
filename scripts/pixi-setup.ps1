$ROOT = git rev-parse --show-toplevel
Set-Location $PSScriptRoot/..
pixi clean
pixi update
pixi install
Set-Location $ROOT
