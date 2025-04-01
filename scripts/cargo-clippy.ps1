$ROOT = git rev-parse --show-toplevel
Set-Location $PSScriptRoot/..
& $PSScriptRoot/setup.ps1

if ($env:CI) {
    cargo +stable clippy --all-features -p fornax
}
else {
    cargo clippy --fix --all-features -p fornax
}

Set-Location $ROOT
