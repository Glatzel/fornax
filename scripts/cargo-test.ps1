$ROOT = git rev-parse --show-toplevel
Set-Location $PSScriptRoot/..
git submodule update --init --recursive
$code = 0

Write-Output "::group::nextest"
cargo +nightly llvm-cov --no-report --all-features --branch nextest --no-fail-fast -p fornax -p dnc -p libraw -p fornax-dalim
$code = $code + $LASTEXITCODE
Write-Output "::endgroup::"

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
    Write-Output "Test succeeded."
}
Write-Output "::endgroup::"
Set-Location $ROOT
exit $code
