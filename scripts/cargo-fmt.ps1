$ROOT = git rev-parse --show-toplevel
Set-Location $PSScriptRoot/..
& $PSScriptRoot/setup.ps1
if ($env:CI) {
    cargo +nightly fmt --all -- --check
}
else {
    cargo +nightly fmt --all
}

Set-Location $ROOT
