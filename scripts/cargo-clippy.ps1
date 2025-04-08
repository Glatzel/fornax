$ROOT = git rev-parse --show-toplevel
Set-Location $PSScriptRoot/..
& $PSScriptRoot/setup.ps1

if ($env:CI) {
    pixi run cargo +stable clippy --all-features --exclude libraw-sys --workspace
}
else {
    pixi run cargo clippy --fix --all-features --exclude libraw-sys --workspace
}

Set-Location $ROOT
