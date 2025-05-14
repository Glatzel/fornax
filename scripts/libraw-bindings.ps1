$ROOT = git rev-parse --show-toplevel
Set-Location $PSScriptRoot/..
$env:RUSTFLAGS = "-Dwarnings"
cargo build  --all-features -p libraw-sys
Set-Location $ROOT
