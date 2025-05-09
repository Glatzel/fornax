param ([switch]$update)
Set-Location $PSScriptRoot/..
pixi run --manifest-path ./pre-commit/pixi.toml pre-commit-run --color=always --show-diff-on-failure
