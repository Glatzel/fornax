$ROOT = git rev-parse --show-toplevel
Set-Location $PSScriptRoot/..
$env:RUSTFLAGS = "-Dwarnings"
& $PSScriptRoot/setup.ps1
cargo build  --all-features -p libraw-sys
Set-Location $ROOT