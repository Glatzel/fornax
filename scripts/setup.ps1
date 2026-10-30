$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot/..

$pythonCommand = Get-Command python -ErrorAction SilentlyContinue
if (-not $pythonCommand)
{
    $pythonCommand = Get-Command python3 -ErrorAction SilentlyContinue
}
if (-not $pythonCommand)
{
    throw "Python 3 is required to run scripts/setup.py"
}

& $pythonCommand.Source $PSScriptRoot/setup.py @args
if ($LASTEXITCODE -ne 0)
{
    exit $LASTEXITCODE
}

$setupEnvironment = (& $pythonCommand.Source $PSScriptRoot/setup.py --env-json | ConvertFrom-Json)
$env:LIBRAW_ROOT = $setupEnvironment.libraw_root
$env:Path = "$($setupEnvironment.path_entry)$([IO.Path]::PathSeparator)$env:Path"
if ($setupEnvironment.loader_name)
{
    $loaderPath = [Environment]::GetEnvironmentVariable($setupEnvironment.loader_name)
    $envValue = if ($loaderPath)
    {
        "$($setupEnvironment.loader_path)$([IO.Path]::PathSeparator)$loaderPath"
    }
    else
    {
        $setupEnvironment.loader_path
    }
    [Environment]::SetEnvironmentVariable($setupEnvironment.loader_name, $envValue)
}
