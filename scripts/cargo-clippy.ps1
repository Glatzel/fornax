$ROOT = git rev-parse --show-toplevel
Set-Location $PSScriptRoot/..
& $PSScriptRoot/setup.ps1

if ($env:CI) {
    pixi run cargo clippy --all-features --workspace --exclude libraw-sys
}
else {
    pixi run cargo clippy --fix --all-features --workspace --exclude libraw-sys
}

Set-Location $ROOT
