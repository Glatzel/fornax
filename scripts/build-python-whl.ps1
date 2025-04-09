param (
    [ValidateSet("develop", "release")]
    $config = "develop"
)
$ROOT = git rev-parse --show-toplevel
& $PSScriptRoot/setup.ps1
Set-Location $PSScriptRoot/../crates/fornax-py
Remove-Item ./dist/fornax*.whl -ErrorAction SilentlyContinue
Remove-Item ./fornax/fornax_py.pyd -ErrorAction SilentlyContinue
Remove-Item ./**__pycache__ -Recurse -ErrorAction SilentlyContinue
pixi run maturin build --out ./dist --profile $config

Set-Location $ROOT
