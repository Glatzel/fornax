# This File is automatically synchronized from https://github.com/Glatzel/template

$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true
$config = if ($args.Count) { $args } else { @('--no-deps', '--workspace', '--all-features') }
if (Test-Path $PSScriptRoot/setup.ps1) { &$PSScriptRoot/setup.ps1 }
Set-Location $PSScriptRoot/..
cargo doc @config
