$ROOT = git rev-parse --show-toplevel
Set-Location $PSScriptRoot/..
& $PSScriptRoot/setup.ps1

if ($env:CI) {
    cargo +stable clippy --all-features
}
else {
    cargo clippy --fix --all-targets --all-features
}

Set-Location $ROOT
