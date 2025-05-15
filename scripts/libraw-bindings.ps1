$ROOT = git rev-parse --show-toplevel
Set-Location $PSScriptRoot/..
$env:RUSTFLAGS = "-Dwarnings"
$env:UPDATE = "true"
cargo build -p libraw-sys
Set-Location $ROOT
