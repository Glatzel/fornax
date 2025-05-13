$ROOT = git rev-parse --show-toplevel
Set-Location $PSScriptRoot/..
cargo +nightly fmt --all
Set-Location $ROOT
