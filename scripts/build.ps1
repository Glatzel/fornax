param (
    [ValidateSet("develop", "release")]
    $config = "develop"
)
$ROOT = git rev-parse --show-toplevel
&"$PSScriptRoot/setup.ps1"
Set-Location $PSScriptRoot/..

cargo build
