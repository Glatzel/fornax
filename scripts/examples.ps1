$ROOT = git rev-parse --show-toplevel
&"$PSScriptRoot/setup.ps1"
Set-Location $PSScriptRoot/..

New-Item temp -ItemType Directory -ErrorAction SilentlyContinue

cargo run --package fornax --example "process"
cargo run --package fornax --example "sizes"