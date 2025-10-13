param(
    [Parameter(Mandatory = $true)]
    [string]$version
)
$ROOT = git rev-parse --show-toplevel
Set-Location $PSScriptRoot/..
if ($IsWindows) {
    if (-not (Test-Path "$ROOT/temp/dnc$version.exe")) {
        Write-Output "::group::download dnc $version"
        aria2c -c -x16 -s16 `
            -d "$ROOT/temp/" `
            "https://download.adobe.com/pub/adobe/dng/win/AdobeDNGConverter_x64_$version.exe" `
            -o "dnc$version.exe"
        Write-Output "::endgroup::"
    }
    Write-Output "::group::install dnc $version"
    Start-Process "./temp/dnc$version.exe" -ArgumentList "/silent" -Wait
    Write-Output "dnc installed"
    Write-Output "::endgroup::"
}
if ($IsMacOS) {
    if (-not (Test-Path "$ROOT/temp/dnc$version.dmg")) {
        Write-Output "::group::download dnc $version"
        aria2c -c -x16 -s16 `
            -d "$ROOT/temp" `
            "https://download.adobe.com/pub/adobe/dng/mac/AdobeDNGConverter_$version.dmg" `
            -o "dnc$version.dmg"
        Write-Output "::endgroup::"
    }
    Write-Output "::group::install dnc $version"
    # Mount the DMG
    $mount = hdiutil attach "$ROOT/temp/dnc$version.dmg" -nobrowse -quiet | Select-String "/Volumes/" | ForEach-Object { $_.ToString().Trim() }
    $appPath = Get-ChildItem "$mount" -Filter "*.app" | Select-Object -First 1

    # Copy to Applications
    if ($appPath) {
        Copy-Item -r $appPath "/Applications/"
    }

    # Unmount
    hdiutil detach "$mount" -quiet
    Write-Output "dnc installed"
    Write-Output "::endgroup::"
}
