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
            -d "$ROOT/temp" `
            "https://download.adobe.com/pub/adobe/dng/win/AdobeDNGConverter_x64_$version.exe" `
            -o "dnc$version.exe"
        Write-Output "::endgroup::"
    }
    Write-Output "::group::install dnc $version"
    Start-Process "./temp/dnc$version.exe" -ArgumentList "/silent" -Wait
    Write-Output "dnc installed"
    Write-Output "::endgroup::"
    $env:Path = "C:\Program Files\Adobe\Adobe DNG Converter;$env:Path"
}
if ($IsMacOS) {
    if (-not (Test-Path "$ROOT/temp/dnc$version.dmg")) {
        Write-Output "::group::download dnc $version"
        aria2c -c -x16 -s16 `
            -d "$ROOT/temp" `
            "https://download.adobe.com/pub/adobe/dng/mac/DNGConverter_$version.dmg" `
            -o "dnc$version.dmg"
        Write-Output "::endgroup::"
    }
    Write-Output "::group::install dnc $version"
    # Path to your DMG
    $dmg = "$ROOT/temp/dnc$version.dmg"

    # Mount the DMG
    $mountInfo = hdiutil attach $dmg -nobrowse
    Write-Output $mountInfo

    # Mounted path from hdiutil attach
    $volume = "/Volumes/DNGConverter_$version"

    # If it contains an .app bundle
    $app = Get-ChildItem -Path $volume -Filter "*.app" | Select-Object -First 1

    if ($app) {
        Write-Host "Installing $($app.Name)..."
        sudo cp -R "$($app.FullName)" /Applications/
        Write-Host "✅ Installed to /Applications"
    }
    else {
        # Maybe it's a .pkg installer
        $pkg = Get-ChildItem -Path $volume -Filter "*.pkg" | Select-Object -First 1
        if ($pkg) {
            Write-Host "Running installer for $($pkg.Name)..."
            sudo installer -pkg "$($pkg.FullName)" -target /Applications/
            Get-ChildItem "/Applications/Adobe DNG Converter.app"
            Write-Host "✅ Package installed"
        }
        else {
            Write-Warning "No .app or .pkg found in $volume"
        }
    }

    # Unmount
    hdiutil detach "$volume"
    Write-Output "dnc installed"
    Write-Output "::endgroup::"

    Write-Output "::group::add environment variable"
    $env:Path = "/Applications/;$env:Path"
    Write-Output "::endgroup::"
}
