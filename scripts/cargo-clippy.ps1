$ROOT = git rev-parse --show-toplevel
Set-Location $PSScriptRoot/..
& $PSScriptRoot/setup.ps1

if ($env:CI) {
    cargo +stable clippy --all-features --exclude libraw-sys --workspace
}
else {
    cargo clippy --fix --all-features --exclude libraw-sys --workspace
}

Set-Location $ROOT
