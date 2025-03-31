Set-Location $PSScriptRoot

Remove-Item vcpkg -Recurse -Force -ErrorAction SilentlyContinue
git clone https://github.com/microsoft/vcpkg.git
./vcpkg/bootstrap-vcpkg.bat
