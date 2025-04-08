param (
    [ValidateSet("develop", "release")]
    $config = "develop"
)
$ROOT = git rev-parse --show-toplevel
& $PSScriptRoot/setup.ps1
Set-Location $PSScriptRoot/..
Remove-Item $ROOT/dist/pyxis*.whl -ErrorAction SilentlyContinue
Remove-Item ./pyxis/pyxis_py.pyd -ErrorAction SilentlyContinue
Remove-Item ./pyxis/**__pycache__ -Recurse -ErrorAction SilentlyContinue
pixi run maturin build --out ./dist --profile $config

Set-Location $ROOT
