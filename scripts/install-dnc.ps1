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

    # Find the mounted volume path (e.g., /Volumes/Adobe DNG Converter)
    $volume = ($mountInfo | Select-String "/Volumes/").Line.Trim()
    Write-Output "Mounted at: $volume"

    # Install the app (copy to /Applications)
    Copy-Item -r "$volume/Adobe DNG Converter.app" /Applications/

    # Unmount
    hdiutil detach "$volume"
    Write-Output "dnc installed"
    Write-Output "::endgroup::"
}
