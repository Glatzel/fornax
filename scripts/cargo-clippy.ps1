$ROOT = git rev-parse --show-toplevel
Set-Location $PSScriptRoot/..
& $PSScriptRoot/setup.ps1
pixi run cargo +stable clippy --fix --all-features --workspace --exclude libraw-sys
Set-Location $ROOT
