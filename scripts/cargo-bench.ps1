# This File is automatically synchronized from https://github.com/Glatzel/template

$config = if (-not $args) { '--workspace' } else { $args }
if (Test-Path $PSScriptRoot/setup.ps1) { &$PSScriptRoot/setup.ps1 }
$ROOT = git rev-parse --show-toplevel
Set-Location $PSScriptRoot/..
cargo bench @config
Set-Location $PSScriptRoot
Set-Location $ROOT
