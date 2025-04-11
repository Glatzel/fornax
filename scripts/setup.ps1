$env:PYTHONPATH = Resolve-Path $PSScriptRoot/../crates/fornax-py
pixi install -e libraw
