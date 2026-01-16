Set-Location $PSScriptRoot/..
$version= "0.0.13"

# Update the version in Cargo.toml
$cargoTomlPath = "./Cargo.toml"
(Get-Content -Path $cargoTomlPath) -replace '^version = .*', "version = `"$version`"" | Set-Content -Path $cargoTomlPath
Write-Host "Updated Rust version to $version"
