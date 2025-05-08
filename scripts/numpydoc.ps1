$ROOT = git rev-parse --show-toplevel
& $PSScriptRoot/setup.ps1
Set-Location $PSScriptRoot/../crates/fornax-py
$files = Get-ChildItem -Path "./fornax/*.py" -Recurse
pixi run -e pydev numpydoc lint $files
Set-Location $ROOT
