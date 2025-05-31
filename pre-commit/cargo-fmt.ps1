$ROOT = git rev-parse --show-toplevel
Set-Location $ROOT
foreach ($f in Get-ChildItem "Cargo.lock" -Recurse) {
    Set-Location $f.Directory.ToString()
    Write-Output "Cargo fmt in: $pwd"
    cargo +nightly fmt --all
}
Set-Location $ROOT
