$ROOT = git rev-parse --show-toplevel
Set-Location $PSScriptRoot/..
& $PSScriptRoot/setup.ps1
cargo +stable clippy --fix --all -- -D warnings
Set-Location $ROOT
