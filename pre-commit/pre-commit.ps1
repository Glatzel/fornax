param ([switch]$update)

$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $true
Set-Location $PSScriptRoot/..
New-Item ./pre-commit -ItemType Directory -ErrorAction SilentlyContinue
if ($update) {
    for ($i = 0; $i -lt 2; $i++) {
        curl -L -o ./pre-commit/clippy.toml  https://raw.githubusercontent.com/Glatzel/template/main//pre-commit/clippy.toml
        curl -L -o ./pre-commit/pixi.lock  https://raw.githubusercontent.com/Glatzel/template/main//pre-commit/pixi.lock
        curl -L -o ./pre-commit/pixi.toml  https://raw.githubusercontent.com/Glatzel/template/main//pre-commit/pixi.toml
        curl -L -o ./pre-commit/pre-commit.ps1  https://raw.githubusercontent.com/Glatzel/template/main//pre-commit/pre-commit.ps1
        curl -L -o ./pre-commit/ruff.toml  https://raw.githubusercontent.com/Glatzel/template/main//pre-commit/ruff.toml
        curl -L -o ./pre-commit/rustfmt.toml  https://raw.githubusercontent.com/Glatzel/template/main//pre-commit/rustfmt.toml
        curl -L -o .pre-commit-config.yaml  https://raw.githubusercontent.com/Glatzel/template/main//.pre-commit-config.yaml
    }
    exit 0
}
pixi install --manifest-path ./pre-commit/pixi.toml
pixi run --manifest-path ./pre-commit/pixi.toml pre-commit-run --color=always --show-diff-on-failure -v
