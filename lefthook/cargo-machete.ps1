# This File is automatically synchronized from https://github.com/Glatzel/template

$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true
$ROOT = git rev-parse --show-toplevel
Set-Location $ROOT
foreach ($file in $args) {
    if ("$file".Contains("target")) { continue }
    if ("$file".Contains("crate")) { continue }
    Set-Location (Split-Path (Resolve-Path $file) -Parent)
    Write-Output "Cargo machete in: $pwd"
    cargo machete
}
Set-Location $ROOT
