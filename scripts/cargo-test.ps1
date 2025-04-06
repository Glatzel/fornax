$ROOT = git rev-parse --show-toplevel
Set-Location $PSScriptRoot/..
git submodule update --init --recursive
& $PSScriptRoot/setup.ps1
$code = 0

Write-Output "::group::nextest"
cargo +nightly llvm-cov --no-report --all-features --all --branch nextest
$code = $code + $LASTEXITCODE
Write-Output "::endgroup::"

# Write-Output "::group::doctest"
# cargo +nightly llvm-cov --no-report --all-features --all --branch --doc
# $code = $code + $LASTEXITCODE
# Write-Output "::endgroup::"

Write-Output "::group::report"
cargo +nightly llvm-cov report
Write-Output "::endgroup::"

Write-Output "::group::lcov"
if ( $env:CI ) {
    cargo +nightly llvm-cov report --lcov --output-path lcov.info
}
Write-Output "::endgroup::"

Write-Output "::group::result"
$code = $code + $LASTEXITCODE
if ($code -ne 0) {
    Write-Output "Test failed."
}
else {
    Write-Output "Test successed."
}
Write-Output "::endgroup::"
Set-Location $ROOT
exit $code
