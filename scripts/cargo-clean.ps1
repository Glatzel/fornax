$ROOT = git rev-parse --show-toplevel
Set-Location $PSScriptRoot/..
cargo clean
Set-Location $ROOT
