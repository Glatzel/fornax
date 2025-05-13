$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true
$ROOT = git rev-parse --show-toplevel
Set-Location $PSScriptRoot/..
$env:RUSTFLAGS= "-Dwarnings"
cargo doc --no-deps --all-features `
    -p dnc `
    -p fornax `
    -p fornax-core `
    -p libraw

Remove-Item ./dist/rust-doc.7z -Force -ErrorAction SilentlyContinue
New-Item ./dist -ItemType Directory -ErrorAction SilentlyContinue
7z a -t7z -m0=LZMA2 -mmt=on -mx9 -md=4096m -mfb=273 -ms=on -mqs=on `
    "./dist/rust-doc.7z" "./target/doc/*"
Set-Location $ROOT
