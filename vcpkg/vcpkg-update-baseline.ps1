Set-Location $PSScriptRoot

./vcpkg-setup.ps1
./vcpkg/vcpkg.exe x-update-baseline
