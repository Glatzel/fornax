if (Test-Path $PSScriptRoot/setup.ps1) {
    &$PSScriptRoot/setup.ps1
}
$ROOT = git rev-parse --show-toplevel
Set-Location $PSScriptRoot/..

Write-Output "::group::nextest"
cargo +nightly llvm-cov nextest --no-report --all --all-features --branch --no-fail-fast
$code = $LASTEXITCODE
Write-Output "::endgroup::"

Write-Output "::group::doctest"
cargo +nightly llvm-cov --no-report --all --all-features --branch --no-fail-fast --doc
$code = $code + $LASTEXITCODE
Write-Output "::endgroup::"

Write-Output "::group::report"
if ( $env:CI ) {
    cargo +nightly llvm-cov report --lcov --output-path lcov.info
}
else {
    cargo +nightly llvm-cov report
    cargo +nightly llvm-cov report --html
}
Write-Output "::endgroup::"

Write-Output "::group::result"
$code = $code + $LASTEXITCODE
if ($code -ne 0) {
    Write-Host "Test failed." -ForegroundColor Red
}
else {
    Write-Host "Test succeeded." -ForegroundColor Green
}
Write-Output "::endgroup::"
Set-Location $ROOT
exit $code
