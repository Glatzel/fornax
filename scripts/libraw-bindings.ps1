& $PSScriptRoot/setup.ps1
Set-Location $PSScriptRoot/..
$env:RUSTFLAGS = "-Dwarnings"
$env:UPDATE = "true"
cargo build -p libraw-sys
Set-Location $ROOT
