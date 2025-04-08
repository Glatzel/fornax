$ROOT = git rev-parse --show-toplevel
& $PSScriptRoot/setup.ps1
Set-Location $PSScriptRoot/../crates/fornax-py
# run test
pixi run -e pydev pytest `
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
