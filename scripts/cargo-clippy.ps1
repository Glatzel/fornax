$ROOT = git rev-parse --show-toplevel
Set-Location $PSScriptRoot/..
pixi run cargo +stable clippy --fix --all -- -D warnings
Set-Location $ROOT
