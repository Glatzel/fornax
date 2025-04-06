param (
    [ValidateSet("develop", "release")]
    $config = "develop"
)
&"$PSScriptRoot/setup.ps1"
Set-Location $PSScriptRoot/..

cargo build --profile $config
