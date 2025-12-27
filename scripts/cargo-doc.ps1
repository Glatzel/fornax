# This File is automatically synchronized from https://github.com/Glatzel/template

if (Test-Path $PSScriptRoot/setup.ps1) { &$PSScriptRoot/setup.ps1 }
$config = if ($args.Count) { $args } else { @('--no-deps', '--workspace', '--all-features') }
Set-Location $PSScriptRoot/..
cargo doc @config
