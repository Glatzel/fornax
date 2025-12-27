# This File is automatically synchronized from https://github.com/Glatzel/template

if (Test-Path $PSScriptRoot/setup.ps1) { &$PSScriptRoot/setup.ps1 }
Set-Location $PSScriptRoot/..
cargo clean
Set-Location $ROOT
