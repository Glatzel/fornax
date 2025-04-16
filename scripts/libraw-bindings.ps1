$ROOT = git rev-parse --show-toplevel
Set-Location $PSScriptRoot/..
$env:RUSTFLAGS = "-Dwarnings"
& $PSScriptRoot/setup.ps1
$env:OUT_DIR=resolve-path "$PSScriptRoot/../crates/libraw-sys/src"
cargo build  --all-features -p libraw-sys
Set-Location $ROOT
