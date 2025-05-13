$ROOT = git rev-parse --show-toplevel
Set-Location $PSScriptRoot/..
cargo +stable clippy --fix --all -- -D warnings
Set-Location $ROOT
