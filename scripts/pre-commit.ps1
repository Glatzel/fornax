param ([switch]$update)

Set-Location $PSScriptRoot/..
New-Item ./pre-commit -ItemType Directory -ErrorAction SilentlyContinue
if ($update) {
    curl -L -o .pre-commit-config.yaml  https://raw.githubusercontent.com/Glatzel/template/main//.pre-commit-config.yaml
    curl -L -o ./pre-commit/pixi.toml  https://raw.githubusercontent.com/Glatzel/template/main//pre-commit/pixi.toml
    curl -L -o ./pre-commit/ruff.toml  https://raw.githubusercontent.com/Glatzel/template/main//pre-commit/ruff.toml
}
pixi install --manifest-path ./pre-commit/pixi.toml
pixi run --manifest-path ./pre-commit/pixi.toml pre-commit-run --color=always --show-diff-on-failure
