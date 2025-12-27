& $PSScriptRoot/setup.ps1
Set-Location $PSScriptRoot/../crates/fornax-py
$env:PYTHONPATH = Resolve-Path $PSScriptRoot/../crates/fornax-py
git submodule update --init --recursive
# run test
pixi run pytest `
    ./tests `
    -v `
    --durations=10 `
    --junitxml=tests_report/junit.xml `
    -o junit_family=legacy `
    --cov `
    --cov-report term `
    --cov-report=xml:tests_report/coverage.xml `
    --cov-report=html:tests_report/htmlcov
Set-Location $ROOT
