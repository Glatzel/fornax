$ROOT = git rev-parse --show-toplevel
Set-Location $PSScriptRoot/..
& $PSScriptRoot/setup.ps1
cargo +nightly fmt --all -- --config-path $ROOT
Set-Location $ROOT
