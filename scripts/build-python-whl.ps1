Set-Location $PSScriptRoot/../crates/fornax-py
Remove-Item ./dist/fornax*.whl -ErrorAction SilentlyContinue
Remove-Item ./fornax/fornax_py.pyd -ErrorAction SilentlyContinue
Remove-Item ./**__pycache__ -Recurse -ErrorAction SilentlyContinue
pixi run maturin build --out ./dist --profile release
