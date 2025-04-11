$ROOT = git rev-parse --show-toplevel
Set-Location $PSScriptRoot/..
& $PSScriptRoot/setup.ps1

if ($env:CI) {
    pixi run -e libraw cargo clippy --all-features --workspace --exclude libraw-sys
}
else {
    pixi run -e libraw cargo clippy --fix --all-features --workspace --exclude libraw-sys
}

Set-Location $ROOT
