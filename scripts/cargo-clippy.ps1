$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true
$ROOT = git rev-parse --show-toplevel
Set-Location $PSScriptRoot/..
cargo +stable clippy --fix --all
cargo +stable clippy -- -Dwarnings
Set-Location $ROOT
