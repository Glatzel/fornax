Set-Location $PSScriptRoot/..
$version= "0.0.6"

# Update the version in Cargo.toml
$cargoTomlPath = "./Cargo.toml"
(Get-Content -Path $cargoTomlPath) -replace '^version = .*', "version = `"$version`"" | Set-Content -Path $cargoTomlPath
Write-Host "Updated Rust version to $version"

# Update python rattler version
$recipe_path = "./crates/fornax-py/rattler/recipe.yaml"
(Get-Content -Path $recipe_path) -replace '^  version: .*', "  version: $version" | Set-Content -Path $recipe_path
Write-Host "Updated ratter version to $version"
