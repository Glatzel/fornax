&"$PSScriptRoot/setup.ps1"
Set-Location $PSScriptRoot/..

New-Item temp -ItemType Directory -ErrorAction SilentlyContinue

git submodule update --init --recursive
cargo run --package fornax --example "dnc"
