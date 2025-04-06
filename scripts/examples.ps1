&"$PSScriptRoot/setup.ps1"
Set-Location $PSScriptRoot/..

New-Item temp -ItemType Directory -ErrorAction SilentlyContinue

git submodule update --init --recursive
cargo run --package fornax --example "dnc"
cargo run --package fornax --example "libraw-imgother"
cargo run --package fornax --example "libraw-iparams"
cargo run --package fornax --example "libraw-process-cg"
cargo run --package fornax --example "libraw-process"
cargo run --package fornax --example "libraw-sizes"