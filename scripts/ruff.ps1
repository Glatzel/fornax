Set-Location $PSScriptRoot/..
pixi run ruff format
pixi run ruff check --fix
