$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true
$ROOT = git rev-parse --show-toplevel
foreach ($f in Get-ChildItem "Cargo.lock" -Recurse) {
    Set-Location $f.Directory.ToString()
    if (Test-Path ./scripts/setup.ps1) {
        &./scripts/setup.ps1
        Set-Location $f.Directory.ToString()
    }
    cargo +stable clippy --fix
    cargo +stable clippy -- -Dwarnings
    Set-Location $ROOT
}
